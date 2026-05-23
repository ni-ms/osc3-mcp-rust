//! Maps AI tool names/values to the real `nih_plug` parameters.
//!
//! Writes are expressed as [`RawParamEvent`]s and emitted by the caller (the
//! chat's background task, via a Vizia `ContextProxy`). This is the same path
//! the GUI knobs use — the host sees every change as a proper automation
//! gesture, and the audio thread picks it up by reading atomics. No mirror, no
//! locks on the audio thread.

use crate::{FilterMode, SineParams, Waveform};
use nih_plug::prelude::Param;
use serde_json::Value;
use vizia_plug::widgets::RawParamEvent;

/// Emit the Begin/Set/End triple that writes one parameter.
pub fn emit_set<P: Param>(param: &P, plain: P::Plain, emit: &mut impl FnMut(RawParamEvent)) {
    let ptr = param.as_ptr();
    emit(RawParamEvent::BeginSetParameter(ptr));
    emit(RawParamEvent::SetParameterNormalized(
        ptr,
        param.preview_normalized(plain),
    ));
    emit(RawParamEvent::EndSetParameter(ptr));
}

/// Read a JSON value as `f32`, accepting both numbers and numeric strings.
fn as_f32(v: &Value) -> Result<f32, String> {
    if let Some(n) = v.as_f64() {
        return Ok(n as f32);
    }
    v.as_str()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .ok_or_else(|| "expected a number".to_string())
}

/// Read a JSON value as `i32`, accepting numbers and numeric strings.
fn as_i32(v: &Value) -> Result<i32, String> {
    if let Some(n) = v.as_i64() {
        return Ok(n as i32);
    }
    if let Some(n) = v.as_f64() {
        return Ok(n.round() as i32);
    }
    v.as_str()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .ok_or_else(|| "expected an integer".to_string())
}

pub fn wave_to_id(w: Waveform) -> &'static str {
    match w {
        Waveform::Sine => "sine",
        Waveform::Square => "square",
        Waveform::Triangle => "triangle",
        Waveform::Sawtooth => "sawtooth",
    }
}

pub fn id_to_wave(s: &str) -> Waveform {
    match s.trim().to_lowercase().as_str() {
        "square" | "sqr" => Waveform::Square,
        "triangle" | "tri" => Waveform::Triangle,
        "sawtooth" | "saw" => Waveform::Sawtooth,
        _ => Waveform::Sine,
    }
}

pub fn mode_to_id(m: FilterMode) -> &'static str {
    match m {
        FilterMode::LowPass => "lowpass",
        FilterMode::HighPass => "highpass",
        FilterMode::BandPass => "bandpass",
        FilterMode::Notch => "notch",
    }
}

pub fn id_to_mode(s: &str) -> FilterMode {
    match s.trim().to_lowercase().replace([' ', '_', '-'], "").as_str() {
        "highpass" | "hp" => FilterMode::HighPass,
        "bandpass" | "bp" => FilterMode::BandPass,
        "notch" => FilterMode::Notch,
        _ => FilterMode::LowPass,
    }
}

fn parse_wave(v: &Value) -> Result<Waveform, String> {
    v.as_str()
        .map(id_to_wave)
        .ok_or_else(|| "expected a waveform name (sine/square/triangle/sawtooth)".to_string())
}

fn parse_mode(v: &Value) -> Result<FilterMode, String> {
    v.as_str()
        .map(id_to_mode)
        .ok_or_else(|| "expected a filter mode (lowpass/highpass/bandpass/notch)".to_string())
}

/// Resolve a `set_parameter` tool call to a parameter write and emit it.
///
/// `name` is the canonical snake-case vocabulary shared with [`read_state`] and
/// the preset files (`frequency1`, `filter_cutoff`, `attack`, ...).
pub fn apply_write(
    p: &SineParams,
    name: &str,
    value: &Value,
    emit: &mut impl FnMut(RawParamEvent),
) -> Result<(), String> {
    match name {
        // --- Oscillator 1 ---
        "waveform1" => emit_set(&p.osc1.waveform, parse_wave(value)?, emit),
        "frequency1" => emit_set(&p.osc1.frequency, as_f32(value)?, emit),
        "detune1" => emit_set(&p.osc1.detune, as_f32(value)?, emit),
        "phase1" => emit_set(&p.osc1.phase, as_f32(value)?, emit),
        "gain1" => emit_set(&p.osc1.gain, as_f32(value)?, emit),
        "octave1" => emit_set(&p.osc1.octave, as_i32(value)?, emit),
        "unison_voices1" => emit_set(&p.osc1.unison_voices, as_i32(value)?, emit),
        "unison_detune1" => emit_set(&p.osc1.unison_detune, as_f32(value)?, emit),
        "unison_blend1" => emit_set(&p.osc1.unison_blend, as_f32(value)?, emit),
        "unison_volume1" => emit_set(&p.osc1.unison_volume, as_f32(value)?, emit),

        // --- Oscillator 2 ---
        "waveform2" => emit_set(&p.osc2.waveform, parse_wave(value)?, emit),
        "frequency2" => emit_set(&p.osc2.frequency, as_f32(value)?, emit),
        "detune2" => emit_set(&p.osc2.detune, as_f32(value)?, emit),
        "phase2" => emit_set(&p.osc2.phase, as_f32(value)?, emit),
        "gain2" => emit_set(&p.osc2.gain, as_f32(value)?, emit),
        "octave2" => emit_set(&p.osc2.octave, as_i32(value)?, emit),
        "unison_voices2" => emit_set(&p.osc2.unison_voices, as_i32(value)?, emit),
        "unison_detune2" => emit_set(&p.osc2.unison_detune, as_f32(value)?, emit),
        "unison_blend2" => emit_set(&p.osc2.unison_blend, as_f32(value)?, emit),
        "unison_volume2" => emit_set(&p.osc2.unison_volume, as_f32(value)?, emit),

        // --- Oscillator 3 ---
        "waveform3" => emit_set(&p.osc3.waveform, parse_wave(value)?, emit),
        "frequency3" => emit_set(&p.osc3.frequency, as_f32(value)?, emit),
        "detune3" => emit_set(&p.osc3.detune, as_f32(value)?, emit),
        "phase3" => emit_set(&p.osc3.phase, as_f32(value)?, emit),
        "gain3" => emit_set(&p.osc3.gain, as_f32(value)?, emit),
        "octave3" => emit_set(&p.osc3.octave, as_i32(value)?, emit),
        "unison_voices3" => emit_set(&p.osc3.unison_voices, as_i32(value)?, emit),
        "unison_detune3" => emit_set(&p.osc3.unison_detune, as_f32(value)?, emit),
        "unison_blend3" => emit_set(&p.osc3.unison_blend, as_f32(value)?, emit),
        "unison_volume3" => emit_set(&p.osc3.unison_volume, as_f32(value)?, emit),

        // --- Filter ---
        "filter_mode" => emit_set(&p.filter.mode, parse_mode(value)?, emit),
        "filter_cutoff" => emit_set(&p.filter.cutoff, as_f32(value)?, emit),
        "filter_resonance" => emit_set(&p.filter.resonance, as_f32(value)?, emit),
        "filter_drive" => emit_set(&p.filter.drive, as_f32(value)?, emit),

        // --- Envelope ---
        "attack" => emit_set(&p.adsr.attack, as_f32(value)?, emit),
        "decay" => emit_set(&p.adsr.decay, as_f32(value)?, emit),
        "sustain" => emit_set(&p.adsr.sustain, as_f32(value)?, emit),
        "release" => emit_set(&p.adsr.release, as_f32(value)?, emit),

        _ => return Err(format!("unknown parameter '{name}'")),
    }
    Ok(())
}

/// Snapshot the live parameter values into the JSON shape the AI sees from the
/// `get_state` tool (the same shape as a preset file's parameter block).
pub fn read_state(p: &SineParams) -> Value {
    serde_json::to_value(crate::ai::preset::PresetData::capture(p))
        .unwrap_or_else(|_| Value::Null)
}
