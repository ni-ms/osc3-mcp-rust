//! OpenRouter configuration and the multi-turn (agentic) tool-calling loop.
//!
//! OpenRouter exposes an OpenAI-compatible `/chat/completions` endpoint, so the
//! request/response shape here is the standard `messages` + `tools` format with
//! a `Bearer` API key, not Gemini's `contents`/`functionCall` schema.

use crate::SineParams;
use crate::ai::tools;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use vizia_plug::vizia::prelude::*;

use super::chat_ui::{ChatEvent, Role};

/// Hard cap on tool-call rounds to avoid runaway chains.
const MAX_ROUNDS: usize = 8;

/// OpenRouter's OpenAI-compatible chat-completions endpoint.
pub const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Default model slug — cheap, fast, and supports both tool calls and audio
/// input (the latter is needed by the Audio Copy "AI Match").
pub const DEFAULT_MODEL: &str = "google/gemini-2.5-flash";

/// Quick-pick model slugs surfaced in settings. Any OpenRouter slug can be typed
/// in by hand; these are just shortcuts. Only tool-call-capable models work.
pub const SUGGESTED_MODELS: &[(&str, &str)] = &[
    ("google/gemini-2.5-flash", "Gemini Flash"),
    ("openai/gpt-4o-mini", "GPT-4o mini"),
    ("anthropic/claude-3.5-sonnet", "Claude Sonnet"),
];

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}
fn default_temperature() -> f32 {
    0.7
}

/// AI settings, persisted to `<config-dir>/TripleOscSynth/config.json` rather
/// than host/project state (so the API key is not embedded in shared projects).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub api_key: String,
    /// OpenRouter model slug, e.g. `openai/gpt-4o`.
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: default_model(),
            temperature: default_temperature(),
        }
    }
}

impl AiConfig {
    fn path() -> std::path::PathBuf {
        crate::ai::preset::app_dir().join("config.json")
    }

    /// Load config. If the file is missing, write out a default one so users
    /// have a documented file to edit (e.g. to paste a key without the GUI). A
    /// present-but-unparseable file is left untouched and defaults are used, so
    /// a hand-edit with a typo isn't silently clobbered.
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::path()) {
            Ok(text) => {
                let mut cfg: Self = serde_json::from_str(&text).unwrap_or_default();
                // An OpenRouter slug always contains a '/'. A bare value (e.g. a
                // leftover Gemini enum like "Gemini25Flash" from an older config)
                // would 404, so fall back to the default instead.
                if !cfg.model.contains('/') {
                    cfg.model = default_model();
                }
                cfg
            }
            Err(_) => {
                let cfg = Self::default();
                let _ = cfg.save();
                cfg
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = crate::ai::preset::app_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("create config dir: {e}"))?;
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(Self::path(), json).map_err(|e| format!("write config: {e}"))
    }
}

fn system_prompt() -> &'static str {
    concat!(
        "You are an expert sound designer embedded in a triple-oscillator subtractive synthesizer. ",
        "Each of the 3 oscillators has a waveform, frequency, detune, phase, gain, octave, and unison ",
        "controls; there is a multimode filter (cutoff/resonance/drive), an ADSR amplitude envelope, ",
        "and a separate ADSR filter envelope whose depth is set by filter_env_amount (in octaves).\n\n",
        "Design sounds by calling set_parameter (call it many times for one request). To tweak or copy ",
        "the existing sound, call get_state first. Save with save_preset, recall with load_preset, and ",
        "use list_presets to discover names. After making changes, reply with a short, friendly summary ",
        "of what you did. Choose musically sensible values within each parameter's stated range."
    )
}

fn role_str(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        _ => "assistant",
    }
}

/// OpenAI tool-call arguments arrive as a JSON *string*; parse it back to a
/// `Value` (tolerating an already-parsed object or a missing/empty field).
fn parse_arguments(raw: &Value) -> Value {
    match raw {
        Value::String(s) => serde_json::from_str(s).unwrap_or_else(|_| json!({})),
        Value::Object(_) => raw.clone(),
        _ => json!({}),
    }
}

/// Run a full request/response exchange, executing any tool calls in-plugin and
/// looping until the model returns a plain-text reply. Always ends by emitting a
/// `ChatEvent::Receive` (success or error) so the UI clears its "sending" state.
pub async fn run_conversation(
    proxy: &mut ContextProxy,
    params: &SineParams,
    cfg: &AiConfig,
    convo: Vec<(Role, String)>,
    cancel: Arc<AtomicBool>,
) {
    let key = cfg.api_key.trim().to_string();
    let model = cfg.model.trim().to_string();

    // Seed the conversation: system prompt + the visible chat history (skip the
    // tool log lines, which aren't part of the model-visible turn structure).
    let mut messages: Vec<Value> = Vec::with_capacity(convo.len() + 1);
    messages.push(json!({ "role": "system", "content": system_prompt() }));
    for (role, text) in convo.iter().filter(|(r, _)| *r != Role::Tool) {
        messages.push(json!({ "role": role_str(*role), "content": text }));
    }

    let client = reqwest::Client::new();

    for _ in 0..MAX_ROUNDS {
        // Bail out between rounds if the user pressed Stop. The UI already reset
        // its "sending" state, so we exit quietly without another Receive.
        if cancel.load(Ordering::Relaxed) {
            return;
        }

        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools::tool_schema(),
            "temperature": cfg.temperature,
        });

        let resp = match client
            .post(OPENROUTER_URL)
            .bearer_auth(&key)
            // Optional attribution headers OpenRouter recommends for ranking.
            .header("HTTP-Referer", "https://github.com/osc3-mcp-rust")
            .header("X-Title", "TripleOscSynth")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = proxy.emit(ChatEvent::Receive(format!("Network error: {e}")));
                return;
            }
        };

        if !resp.status().is_success() {
            let code = resp.status();
            let detail = resp.text().await.unwrap_or_default();
            let detail = detail.chars().take(300).collect::<String>();
            let msg = match code.as_u16() {
                401 => format!(
                    "Unauthorized (401): check your OpenRouter API key in settings (⚙).\n\n{detail}"
                ),
                402 => format!(
                    "Payment required (402): this OpenRouter model needs credits on \
                     your account, or pick a free/cheaper model in settings.\n\n{detail}"
                ),
                429 => format!(
                    "Rate limited (429): OpenRouter is throttling this key or model. \
                     Wait a moment and retry, or switch models in settings.\n\n{detail}"
                ),
                _ => format!("API error {code}: {detail}"),
            };
            let _ = proxy.emit(ChatEvent::Receive(msg));
            return;
        }

        let payload: Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => {
                let _ = proxy.emit(ChatEvent::Receive(format!("Could not parse response: {e}")));
                return;
            }
        };

        let message = payload["choices"][0]["message"].clone();
        let tool_calls = message["tool_calls"].as_array().cloned().unwrap_or_default();
        let text = message["content"].as_str().unwrap_or("").to_string();

        if tool_calls.is_empty() {
            let reply = if text.trim().is_empty() {
                "Done.".to_string()
            } else {
                text
            };
            let _ = proxy.emit(ChatEvent::Receive(reply));
            return;
        }

        // Don't apply a fresh batch of parameter writes if the user stopped
        // while the request was in flight.
        if cancel.load(Ordering::Relaxed) {
            return;
        }

        // Echo the assistant's tool-call turn verbatim, then append one tool
        // result message per call (matched back by tool_call_id).
        messages.push(message);

        for call in &tool_calls {
            let id = call["id"].as_str().unwrap_or_default();
            let name = call["function"]["name"].as_str().unwrap_or_default();
            let args = parse_arguments(&call["function"]["arguments"]);
            let result = tools::dispatch(proxy, params, name, &args);
            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": result.to_string(),
            }));
        }
    }

    let _ = proxy.emit(ChatEvent::Receive(
        "Stopped after too many tool calls — try a more specific request.".to_string(),
    ));
}
