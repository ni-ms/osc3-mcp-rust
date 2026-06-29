//! Gemini "match": send an isolated stem as inline audio and let the model
//! design a patch by calling `set_parameter`, reusing the AI tab's tool schema
//! and in-plugin dispatcher.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine;
use serde_json::{Value, json};
use vizia_plug::vizia::prelude::*;

use crate::SineParams;
use crate::ai::llm::AiConfig;
use crate::ai::tools;

use super::separate::Stem;
use super::ui::AudioCopyEvent;

/// Tool-call rounds cap (mirrors the chat loop).
const MAX_ROUNDS: usize = 6;
/// Gemini prefers 16 kHz mono audio; resampling also shrinks the payload.
const UPLOAD_SR: u32 = 16_000;

fn system_prompt() -> &'static str {
    concat!(
        "You are an expert sound designer embedded in a triple-oscillator subtractive synth. ",
        "You are given one ISOLATED sound from a track. Recreate its TIMBRE on the synth by ",
        "calling set_parameter repeatedly. The synth is pitched by incoming MIDI notes (the osc ",
        "frequency param is a 440 Hz ratio), so do NOT try to match absolute pitch — focus on ",
        "waveform, filter (cutoff/resonance/mode/drive), the amplitude ADSR, the filter envelope, ",
        "and unison width. Use get_state if helpful. When finished, reply with a one-line summary ",
        "of the patch you built."
    )
}

/// Run the Gemini match for `stem`. Always ends by emitting `MatchDone` so the UI
/// clears its busy state.
pub async fn match_gemini(
    proxy: &mut ContextProxy,
    params: &SineParams,
    cfg: &AiConfig,
    stem: &Stem,
    cancel: Arc<AtomicBool>,
) {
    if cfg.api_key.trim().is_empty() {
        let _ = proxy.emit(AudioCopyEvent::MatchDone(
            "Set your Gemini API key in the AI ASSIST tab's settings (⚙) first.".to_string(),
        ));
        return;
    }

    let wav = encode_wav_mono(&stem.samples, stem.sample_rate, UPLOAD_SR);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&wav);

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        cfg.model.api_name(),
        cfg.api_key.trim()
    );

    let prompt = format!(
        "Here is an isolated sound (label: \"{}\"). Recreate its timbre on the synth.",
        stem.label
    );

    let mut contents: Vec<Value> = vec![json!({
        "role": "user",
        "parts": [
            { "text": prompt },
            { "inline_data": { "mime_type": "audio/wav", "data": b64 } }
        ]
    })];

    let client = reqwest::Client::new();

    for _ in 0..MAX_ROUNDS {
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
                let _ = proxy.emit(AudioCopyEvent::MatchDone(format!("Network error: {e}")));
                return;
            }
        };

        if !resp.status().is_success() {
            let code = resp.status();
            let detail = resp.text().await.unwrap_or_default();
            let detail = detail.chars().take(300).collect::<String>();
            let _ = proxy.emit(AudioCopyEvent::MatchDone(format!("API error {code}: {detail}")));
            return;
        }

        let payload: Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => {
                let _ = proxy.emit(AudioCopyEvent::MatchDone(format!(
                    "Could not parse response: {e}"
                )));
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
                format!("Matched \"{}\".", stem.label)
            } else {
                text
            };
            let _ = proxy.emit(AudioCopyEvent::MatchDone(reply));
            return;
        }

        if cancel.load(Ordering::Relaxed) {
            return;
        }

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

    let _ = proxy.emit(AudioCopyEvent::MatchDone(
        "Stopped after too many tool calls.".to_string(),
    ));
}

/// Encode a mono `f32` buffer to a 16-bit PCM WAV byte vector, resampling to
/// `out_sr` (linear interpolation) to keep the upload small.
fn encode_wav_mono(samples: &[f32], in_sr: u32, out_sr: u32) -> Vec<u8> {
    let in_sr = in_sr.max(1);
    let out_sr = out_sr.max(1);

    // Resample to out_sr.
    let resampled: Vec<f32> = if in_sr == out_sr {
        samples.to_vec()
    } else {
        let ratio = out_sr as f32 / in_sr as f32;
        let out_len = ((samples.len() as f32) * ratio) as usize;
        (0..out_len)
            .map(|j| {
                let pos = j as f32 / ratio;
                let i0 = pos.floor() as usize;
                let frac = pos - i0 as f32;
                let a = samples.get(i0).copied().unwrap_or(0.0);
                let b = samples.get(i0 + 1).copied().unwrap_or(a);
                a + (b - a) * frac
            })
            .collect()
    };

    let num_samples = resampled.len() as u32;
    let byte_rate = out_sr * 2; // mono, 16-bit
    let data_bytes = num_samples * 2;

    let mut out = Vec::with_capacity(44 + data_bytes as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&out_sr.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for s in resampled {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}
