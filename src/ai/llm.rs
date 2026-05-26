//! Gemini configuration and the multi-turn (agentic) tool-calling loop.

use crate::ai::{preset, tools};
use crate::SineParams;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;

use super::chat_ui::{ChatEvent, Role};

/// Hard cap on tool-call rounds to avoid runaway chains.
const MAX_ROUNDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Data)]
pub enum AiModel {
    Gemini25Flash,
    Gemini25Pro,
    Gemini20Flash,
}

impl AiModel {
    pub fn api_name(&self) -> &'static str {
        match self {
            AiModel::Gemini25Flash => "gemini-2.5-flash",
            AiModel::Gemini25Pro => "gemini-2.5-pro",
            AiModel::Gemini20Flash => "gemini-2.0-flash",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AiModel::Gemini25Flash => "2.5 Flash",
            AiModel::Gemini25Pro => "2.5 Pro",
            AiModel::Gemini20Flash => "2.0 Flash",
        }
    }
}

/// AI settings, persisted to `<config-dir>/TripleOscSynth/config.json` rather
/// than host/project state (so the API key is not embedded in shared projects).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub api_key: String,
    pub model: AiModel,
    pub temperature: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: AiModel::Gemini25Flash,
            temperature: 0.7,
        }
    }
}

impl AiConfig {
    fn path() -> std::path::PathBuf {
        preset::app_dir().join("config.json")
    }

    /// Load config. If the file is missing, write out a default one so users
    /// have a documented file to edit (e.g. to paste a key without the GUI). A
    /// present-but-unparseable file is left untouched and defaults are used, so
    /// a hand-edit with a typo isn't silently clobbered.
    pub fn load() -> Self {
        match std::fs::read_to_string(Self::path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => {
                let cfg = Self::default();
                let _ = cfg.save();
                cfg
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = preset::app_dir();
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
        _ => "model",
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
    // Keys copied from AI Studio often carry a trailing newline or surrounding
    // whitespace; left in the URL that yields an opaque API error, so trim it.
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        cfg.model.api_name(),
        cfg.api_key.trim()
    );

    // Seed the conversation with the visible chat history (skip tool log lines).
    let mut contents: Vec<Value> = convo
        .iter()
        .filter(|(role, _)| *role != Role::Tool)
        .map(|(role, text)| json!({ "role": role_str(*role), "parts": [{ "text": text }] }))
        .collect();

    let client = reqwest::Client::new();

    for _ in 0..MAX_ROUNDS {
        // Bail out between rounds if the user pressed Stop. The UI already reset
        // its "sending" state, so we exit quietly without another Receive.
        if cancel.load(Ordering::Relaxed) {
            return;
        }

        let body = json!({
            "system_instruction": { "parts": [{ "text": system_prompt() }] },
            "contents": contents,
            "tools": tools::gemini_tools(),
            "generationConfig": { "temperature": cfg.temperature }
        });

        let resp = match client.post(&url).json(&body).send().await {
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
            // 429 is a rate-limit/quota response, not a config error — say so
            // plainly so it isn't mistaken for a bad key.
            let msg = if code.as_u16() == 429 {
                format!(
                    "Rate limited (429): you've hit Gemini's per-minute or daily \
                     quota. Wait a minute and retry, switch to a lighter model \
                     (2.0 Flash), or enable billing on the API key's project.\n\n{detail}"
                )
            } else {
                format!("API error {code}: {detail}")
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

        let content = payload["candidates"][0]["content"].clone();
        let parts = content["parts"].as_array().cloned().unwrap_or_default();

        let mut calls: Vec<Value> = Vec::new();
        let mut text = String::new();
        for part in &parts {
            if let Some(fc) = part.get("functionCall") {
                calls.push(fc.clone());
            } else if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                text.push_str(t);
            }
        }

        if calls.is_empty() {
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

        // Echo the model's tool-call turn, then append our results.
        contents.push(content);

        let mut response_parts = Vec::with_capacity(calls.len());
        for fc in &calls {
            let name = fc.get("name").and_then(|n| n.as_str()).unwrap_or_default();
            let args = fc.get("args").cloned().unwrap_or_else(|| json!({}));
            let result = tools::dispatch(proxy, params, name, &args);
            response_parts.push(json!({
                "functionResponse": { "name": name, "response": { "result": result } }
            }));
        }
        contents.push(json!({ "role": "user", "parts": response_parts }));
    }

    let _ = proxy.emit(ChatEvent::Receive(
        "Stopped after too many tool calls — try a more specific request.".to_string(),
    ));
}
