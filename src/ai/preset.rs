//! Preset capture/apply and on-disk JSON storage.
//!
//! [`PresetData`] is a flat, serializable snapshot of every synth parameter. It
//! is the format for `presets/<name>.json` files and the payload returned by the
//! `get_state` tool. `capture` reads the live parameters; `apply` writes them
//! back by emitting [`RawParamEvent`]s.

use crate::ai::bridge::{emit_set, id_to_mode, id_to_wave, mode_to_id, wave_to_id};
use crate::SineParams;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vizia_plug::widgets::RawParamEvent;

const SCHEMA_VERSION: u32 = 1;

/// A complete, serializable snapshot of the synth's parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetData {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub schema_version: u32,

    // --- Oscillator 1 ---
    pub waveform1: String,
    pub frequency1: f32,
    pub detune1: f32,
    pub phase1: f32,
    pub gain1: f32,
    pub octave1: i32,
    pub unison_voices1: i32,
    pub unison_detune1: f32,
    pub unison_blend1: f32,
    pub unison_volume1: f32,

    // --- Oscillator 2 ---
    pub waveform2: String,
    pub frequency2: f32,
    pub detune2: f32,
    pub phase2: f32,
    pub gain2: f32,
    pub octave2: i32,
    pub unison_voices2: i32,
    pub unison_detune2: f32,
    pub unison_blend2: f32,
    pub unison_volume2: f32,

    // --- Oscillator 3 ---
    pub waveform3: String,
    pub frequency3: f32,
    pub detune3: f32,
    pub phase3: f32,
    pub gain3: f32,
    pub octave3: i32,
    pub unison_voices3: i32,
    pub unison_detune3: f32,
    pub unison_blend3: f32,
    pub unison_volume3: f32,

    // --- Filter ---
    pub filter_mode: String,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_drive: f32,

    // --- Envelope (ADSR) ---
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl PresetData {
    /// Read the live parameter values into a snapshot.
    pub fn capture(p: &SineParams) -> Self {
        Self {
            name: String::new(),
            schema_version: SCHEMA_VERSION,

            waveform1: wave_to_id(p.osc1.waveform.value()).into(),
            frequency1: p.osc1.frequency.value(),
            detune1: p.osc1.detune.value(),
            phase1: p.osc1.phase.value(),
            gain1: p.osc1.gain.value(),
            octave1: p.osc1.octave.value(),
            unison_voices1: p.osc1.unison_voices.value(),
            unison_detune1: p.osc1.unison_detune.value(),
            unison_blend1: p.osc1.unison_blend.value(),
            unison_volume1: p.osc1.unison_volume.value(),

            waveform2: wave_to_id(p.osc2.waveform.value()).into(),
            frequency2: p.osc2.frequency.value(),
            detune2: p.osc2.detune.value(),
            phase2: p.osc2.phase.value(),
            gain2: p.osc2.gain.value(),
            octave2: p.osc2.octave.value(),
            unison_voices2: p.osc2.unison_voices.value(),
            unison_detune2: p.osc2.unison_detune.value(),
            unison_blend2: p.osc2.unison_blend.value(),
            unison_volume2: p.osc2.unison_volume.value(),

            waveform3: wave_to_id(p.osc3.waveform.value()).into(),
            frequency3: p.osc3.frequency.value(),
            detune3: p.osc3.detune.value(),
            phase3: p.osc3.phase.value(),
            gain3: p.osc3.gain.value(),
            octave3: p.osc3.octave.value(),
            unison_voices3: p.osc3.unison_voices.value(),
            unison_detune3: p.osc3.unison_detune.value(),
            unison_blend3: p.osc3.unison_blend.value(),
            unison_volume3: p.osc3.unison_volume.value(),

            filter_mode: mode_to_id(p.filter.mode.value()).into(),
            filter_cutoff: p.filter.cutoff.value(),
            filter_resonance: p.filter.resonance.value(),
            filter_drive: p.filter.drive.value(),

            attack: p.adsr.attack.value(),
            decay: p.adsr.decay.value(),
            sustain: p.adsr.sustain.value(),
            release: p.adsr.release.value(),
        }
    }

    /// Apply this snapshot to the live parameters by emitting `RawParamEvent`s.
    pub fn apply(&self, p: &SineParams, emit: &mut impl FnMut(RawParamEvent)) {
        emit_set(&p.osc1.waveform, id_to_wave(&self.waveform1), emit);
        emit_set(&p.osc1.frequency, self.frequency1, emit);
        emit_set(&p.osc1.detune, self.detune1, emit);
        emit_set(&p.osc1.phase, self.phase1, emit);
        emit_set(&p.osc1.gain, self.gain1, emit);
        emit_set(&p.osc1.octave, self.octave1, emit);
        emit_set(&p.osc1.unison_voices, self.unison_voices1, emit);
        emit_set(&p.osc1.unison_detune, self.unison_detune1, emit);
        emit_set(&p.osc1.unison_blend, self.unison_blend1, emit);
        emit_set(&p.osc1.unison_volume, self.unison_volume1, emit);

        emit_set(&p.osc2.waveform, id_to_wave(&self.waveform2), emit);
        emit_set(&p.osc2.frequency, self.frequency2, emit);
        emit_set(&p.osc2.detune, self.detune2, emit);
        emit_set(&p.osc2.phase, self.phase2, emit);
        emit_set(&p.osc2.gain, self.gain2, emit);
        emit_set(&p.osc2.octave, self.octave2, emit);
        emit_set(&p.osc2.unison_voices, self.unison_voices2, emit);
        emit_set(&p.osc2.unison_detune, self.unison_detune2, emit);
        emit_set(&p.osc2.unison_blend, self.unison_blend2, emit);
        emit_set(&p.osc2.unison_volume, self.unison_volume2, emit);

        emit_set(&p.osc3.waveform, id_to_wave(&self.waveform3), emit);
        emit_set(&p.osc3.frequency, self.frequency3, emit);
        emit_set(&p.osc3.detune, self.detune3, emit);
        emit_set(&p.osc3.phase, self.phase3, emit);
        emit_set(&p.osc3.gain, self.gain3, emit);
        emit_set(&p.osc3.octave, self.octave3, emit);
        emit_set(&p.osc3.unison_voices, self.unison_voices3, emit);
        emit_set(&p.osc3.unison_detune, self.unison_detune3, emit);
        emit_set(&p.osc3.unison_blend, self.unison_blend3, emit);
        emit_set(&p.osc3.unison_volume, self.unison_volume3, emit);

        emit_set(&p.filter.mode, id_to_mode(&self.filter_mode), emit);
        emit_set(&p.filter.cutoff, self.filter_cutoff, emit);
        emit_set(&p.filter.resonance, self.filter_resonance, emit);
        emit_set(&p.filter.drive, self.filter_drive, emit);

        emit_set(&p.adsr.attack, self.attack, emit);
        emit_set(&p.adsr.decay, self.decay, emit);
        emit_set(&p.adsr.sustain, self.sustain, emit);
        emit_set(&p.adsr.release, self.release, emit);
    }
}

// --- Disk storage -----------------------------------------------------------

/// `<config-dir>/TripleOscSynth`, falling back to `./TripleOscSynth` if the OS
/// config dir is unavailable.
pub fn app_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("TripleOscSynth")
}

pub fn presets_dir() -> PathBuf {
    app_dir().join("presets")
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
            c
        } else {
            '_'
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Capture the current params and write `presets/<name>.json`.
pub fn save(p: &SineParams, name: &str) -> Result<PathBuf, String> {
    let dir = presets_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create presets dir: {e}"))?;

    let mut data = PresetData::capture(p);
    data.name = name.to_string();

    let path = dir.join(format!("{}.json", sanitize(name)));
    let json = serde_json::to_string_pretty(&data).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Read `presets/<name>.json` into a [`PresetData`].
pub fn load(name: &str) -> Result<PresetData, String> {
    let path = presets_dir().join(format!("{}.json", sanitize(name)));
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let data: PresetData =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    if data.schema_version > SCHEMA_VERSION {
        return Err(format!(
            "preset '{name}' uses schema version {} but this build supports up to {SCHEMA_VERSION}",
            data.schema_version
        ));
    }
    Ok(data)
}

/// Names (file stems) of all saved presets.
pub fn list() -> Vec<String> {
    let dir = presets_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) == Some("json") {
                path.file_stem().map(|s| s.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}
