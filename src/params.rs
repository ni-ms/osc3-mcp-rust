//! Plugin parameters — the single source of truth for all knob values.
//!
//! `nih_plug` stores each parameter's value in atomics, so `Arc<SineParams>` is
//! shared lock-free between the GUI thread and the audio thread. The three
//! oscillators share one `OscillatorParams` definition via `#[nested]`.

use nih_plug::prelude::*;
use std::sync::Arc;
use vizia_plug::ViziaState;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    #[id = "lowpass"]
    LowPass,
    #[id = "highpass"]
    HighPass,
    #[id = "bandpass"]
    BandPass,
    #[id = "notch"]
    Notch,
}

impl Default for FilterMode {
    fn default() -> Self {
        Self::LowPass
    }
}

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    #[id = "sine"]
    Sine,
    #[id = "square"]
    Square,
    #[id = "triangle"]
    Triangle,
    #[id = "sawtooth"]
    Sawtooth,
}

impl Default for Waveform {
    fn default() -> Self {
        Self::Sine
    }
}

/// One oscillator's parameters. Nested three times in [`SineParams`]; the
/// `id_prefix` on each `#[nested]` keeps host automation IDs unique
/// (`osc1_freq`, `osc2_freq`, ...).
#[derive(Params)]
pub struct OscillatorParams {
    #[id = "waveform"]
    pub waveform: EnumParam<Waveform>,
    #[id = "freq"]
    pub frequency: FloatParam,
    #[id = "detune"]
    pub detune: FloatParam,
    #[id = "phase"]
    pub phase: FloatParam,
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "octave"]
    pub octave: IntParam,
    #[id = "unison_voices"]
    pub unison_voices: IntParam,
    #[id = "unison_detune"]
    pub unison_detune: FloatParam,
    #[id = "unison_blend"]
    pub unison_blend: FloatParam,
    #[id = "unison_volume"]
    pub unison_volume: FloatParam,
}

impl OscillatorParams {
    /// Build an oscillator with per-instance defaults. All three oscillators
    /// share identical ranges, smoothing, and formatting.
    fn new(
        default_wave: Waveform,
        default_freq: f32,
        default_gain_db: f32,
        default_octave: i32,
    ) -> Self {
        Self {
            waveform: EnumParam::new("Waveform", default_wave),
            frequency: FloatParam::new(
                "Frequency",
                default_freq,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20_000.0,
                    factor: 0.5,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),

            detune: FloatParam::new(
                "Detune",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit(" cents")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            phase: FloatParam::new("Phase", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(50.0))
                .with_unit("")
                .with_value_to_string(Arc::new(|value| format!("{:.0}°", value * 360.0)))
                .with_string_to_value(Arc::new(|string| {
                    string
                        .trim_end_matches('°')
                        .parse()
                        .ok()
                        .map(|x: f32| x / 360.0)
                })),

            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(default_gain_db),
                FloatRange::Linear {
                    min: util::db_to_gain(-36.0),
                    max: util::db_to_gain(0.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            octave: IntParam::new("Octave", default_octave, IntRange::Linear { min: -4, max: 4 }),

            unison_voices: IntParam::new("Unison Voices", 1, IntRange::Linear { min: 1, max: 8 })
                .with_unit(" voices"),
            unison_detune: FloatParam::new(
                "Unison Detune",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit(" cents"),
            unison_blend: FloatParam::new(
                "Unison Blend",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_percentage(1)),
            unison_volume: FloatParam::new(
                "Unison Volume",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_percentage(1)),
        }
    }
}

#[derive(Params)]
pub struct FilterParams {
    #[id = "mode"]
    pub mode: EnumParam<FilterMode>,
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "resonance"]
    pub resonance: FloatParam,
    #[id = "drive"]
    pub drive: FloatParam,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            mode: EnumParam::new("Filter Mode", FilterMode::LowPass),
            cutoff: FloatParam::new(
                "Filter Cutoff",
                20000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            resonance: FloatParam::new(
                "Filter Resonance",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),

            drive: FloatParam::new(
                "Filter Drive",
                1.0,
                FloatRange::Linear { min: 1.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0)),
        }
    }
}

#[derive(Params)]
pub struct AdsrParams {
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "release"]
    pub release: FloatParam,
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack: FloatParam::new(
                "Attack",
                0.01,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(3)),

            decay: FloatParam::new(
                "Decay",
                0.5,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" s"),

            sustain: FloatParam::new("Sustain", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(10.0))
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(1)),

            release: FloatParam::new(
                "Release",
                1.0,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 10.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" s"),
        }
    }
}

#[derive(Params)]
pub struct SineParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[nested(id_prefix = "osc1", group = "Oscillator 1")]
    pub osc1: OscillatorParams,
    #[nested(id_prefix = "osc2", group = "Oscillator 2")]
    pub osc2: OscillatorParams,
    #[nested(id_prefix = "osc3", group = "Oscillator 3")]
    pub osc3: OscillatorParams,

    #[nested(id_prefix = "filter", group = "Filter")]
    pub filter: FilterParams,

    #[nested(group = "Envelope")]
    pub adsr: AdsrParams,
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            editor_state: crate::ui::editor::default_state(),

            osc1: OscillatorParams::new(Waveform::Sine, 440.0, -6.0, 0),
            osc2: OscillatorParams::new(Waveform::Sawtooth, 880.0, -12.0, -1),
            osc3: OscillatorParams::new(Waveform::Square, 220.0, -18.0, 1),

            filter: FilterParams::default(),
            adsr: AdsrParams::default(),
        }
    }
}
