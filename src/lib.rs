use nih_plug::midi::{MidiConfig, NoteEvent};
use nih_plug::prelude::*;

use std::f32::consts::TAU;
use std::num::NonZeroU32;
use std::sync::Arc;
use vizia_plug::ViziaState;

mod editor;
mod knob;
mod tab_switcher;
mod mcp_server;

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


#[derive(Clone)]
struct UnisonOscillator {
    voices: Vec<OscillatorVoice>,
    num_voices: usize,
}

#[derive(Clone)]
struct OscillatorVoice {
    phase: f32,
    detune_offset: f32,
}

impl UnisonOscillator {
    fn new(max_voices: usize) -> Self {
        let mut voices = Vec::with_capacity(max_voices);
        for i in 0..max_voices {
            let detune_offset = if max_voices == 1 {
                0.0
            } else {
                
                (i as f32 - (max_voices - 1) as f32 / 2.0) / ((max_voices - 1) as f32 / 2.0)
            };
            voices.push(OscillatorVoice {
                phase: 0.0,
                detune_offset,
            });
        }

        Self {
            voices,
            num_voices: 1,
        }
    }

    fn set_num_voices(&mut self, num_voices: usize) {
        self.num_voices = num_voices.min(self.voices.len()).max(1);

        
        for (i, voice) in self.voices.iter_mut().enumerate() {
            voice.detune_offset = if self.num_voices == 1 {
                0.0
            } else {
                (i as f32 - (self.num_voices - 1) as f32 / 2.0)
                    / ((self.num_voices - 1) as f32 / 2.0)
            };
        }
    }

    fn process(
        &mut self,
        waveform: Waveform,
        base_freq: f32,
        detune_cents: f32,
        phase_offset: f32,
        blend: f32,
        volume: f32,
        sample_rate: f32,
    ) -> f32 {
        if self.num_voices == 1 {
            
            let phase_incr = base_freq / sample_rate * TAU;
            let current_phase = self.voices[0].phase + phase_offset * TAU;
            let sample = Self::generate_waveform(waveform, current_phase);

            self.voices[0].phase += phase_incr;
            if self.voices[0].phase >= TAU {
                self.voices[0].phase -= TAU;
            }

            return sample * volume;
        }

        let mut unison_sum = 0.0;
        let mut mono_sample = 0.0;

        for i in 0..self.num_voices {
            let voice = &mut self.voices[i];

            
            let detune_factor = 2.0_f32.powf(voice.detune_offset * detune_cents / 1200.0);
            let detuned_freq = base_freq * detune_factor;
            let phase_incr = detuned_freq / sample_rate * TAU;

            let current_phase = voice.phase + phase_offset * TAU;
            let sample = Self::generate_waveform(waveform, current_phase);

            if i == 0 {
                mono_sample = sample; 
            }

            unison_sum += sample;

            voice.phase += phase_incr;
            if voice.phase >= TAU {
                voice.phase -= TAU;
            }
        }

        let unison_sample = unison_sum / self.num_voices as f32;
        let final_sample = mono_sample * (1.0 - blend) + unison_sample * blend;

        final_sample * volume
    }

    fn generate_waveform(waveform: Waveform, phase: f32) -> f32 {
        match waveform {
            Waveform::Sine => phase.sin(),
            Waveform::Square => {
                if (phase % TAU) < std::f32::consts::PI {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => {
                let normalized_phase = (phase % TAU) / TAU;
                if normalized_phase < 0.5 {
                    4.0 * normalized_phase - 1.0
                } else {
                    3.0 - 4.0 * normalized_phase
                }
            }
            Waveform::Sawtooth => 2.0 * ((phase % TAU) / TAU) - 1.0,
        }
    }

    fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.phase = 0.0;
        }
    }
}


#[derive(Clone)]
struct BiquadFilter {
    
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,

    
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,

    sample_rate: f32,
}

impl BiquadFilter {
    fn new(sample_rate: f32) -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            sample_rate,
        }
    }

    fn set_coefficients(&mut self, mode: FilterMode, cutoff: f32, resonance: f32) {
        let cutoff = cutoff.clamp(20.0, self.sample_rate * 0.49);
        let q = (resonance * 10.0 + 0.5).max(0.1); 

        let omega = 2.0 * std::f32::consts::PI * cutoff / self.sample_rate;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let alpha = sin_omega / (2.0 * q);

        match mode {
            FilterMode::LowPass => {
                let norm = 1.0 + alpha;
                self.b0 = (1.0 - cos_omega) / 2.0 / norm;
                self.b1 = (1.0 - cos_omega) / norm;
                self.b2 = (1.0 - cos_omega) / 2.0 / norm;
                self.a1 = -2.0 * cos_omega / norm;
                self.a2 = (1.0 - alpha) / norm;
            }
            FilterMode::HighPass => {
                let norm = 1.0 + alpha;
                self.b0 = (1.0 + cos_omega) / 2.0 / norm;
                self.b1 = -(1.0 + cos_omega) / norm;
                self.b2 = (1.0 + cos_omega) / 2.0 / norm;
                self.a1 = -2.0 * cos_omega / norm;
                self.a2 = (1.0 - alpha) / norm;
            }
            FilterMode::BandPass => {
                let norm = 1.0 + alpha;
                self.b0 = alpha / norm;
                self.b1 = 0.0;
                self.b2 = -alpha / norm;
                self.a1 = -2.0 * cos_omega / norm;
                self.a2 = (1.0 - alpha) / norm;
            }
            FilterMode::Notch => {
                let norm = 1.0 + alpha;
                self.b0 = 1.0 / norm;
                self.b1 = -2.0 * cos_omega / norm;
                self.b2 = 1.0 / norm;
                self.a1 = -2.0 * cos_omega / norm;
                self.a2 = (1.0 - alpha) / norm;
            }
        }
    }

    fn process(&mut self, input: f32, drive: f32) -> f32 {
        
        let driven_input = if drive > 1.0 {
            (input * drive).tanh() / drive.tanh()
        } else {
            input * drive
        };

        
        let output = self.b0 * driven_input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        
        self.x2 = self.x1;
        self.x1 = driven_input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.reset();
    }
}


#[derive(Clone, Debug, PartialEq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone)]
struct Envelope {
    stage: EnvelopeStage,
    current_level: f32,
    sample_rate: f32,
    samples_elapsed: u32,
    release_start_level: f32,
}

impl Envelope {
    fn new(sample_rate: f32) -> Self {
        Self {
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
            sample_rate,
            samples_elapsed: 0,
            release_start_level: 0.0,
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    fn note_on(&mut self) {
        self.stage = EnvelopeStage::Attack;
        self.samples_elapsed = 0;
    }

    fn note_off(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.release_start_level = self.current_level;
            self.stage = EnvelopeStage::Release;
            self.samples_elapsed = 0;
        }
    }

    fn process(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeStage::Attack => {
                let attack_samples = (attack * self.sample_rate).max(1.0) as u32;
                if self.samples_elapsed >= attack_samples {
                    self.current_level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                    self.samples_elapsed = 0;
                } else {
                    let progress = self.samples_elapsed as f32 / attack_samples as f32;
                    self.current_level = 1.0 - (-5.0 * progress).exp();
                }
            }
            EnvelopeStage::Decay => {
                let decay_samples = (decay * self.sample_rate).max(1.0) as u32;
                if self.samples_elapsed >= decay_samples {
                    self.current_level = sustain;
                    self.stage = EnvelopeStage::Sustain;
                    self.samples_elapsed = 0;
                } else {
                    let progress = self.samples_elapsed as f32 / decay_samples as f32;
                    self.current_level = sustain + (1.0 - sustain) * (-5.0 * progress).exp();
                }
            }
            EnvelopeStage::Sustain => {
                self.current_level = sustain;
            }
            EnvelopeStage::Release => {
                let release_samples = (release * self.sample_rate).max(1.0) as u32;
                if self.samples_elapsed >= release_samples {
                    self.current_level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                    self.samples_elapsed = 0;
                } else {
                    let progress = self.samples_elapsed as f32 / release_samples as f32;
                    self.current_level = self.release_start_level * (-5.0 * progress).exp();
                }
            }
        }

        self.samples_elapsed += 1;
        self.current_level
    }

    fn is_active(&self) -> bool {
        self.stage != EnvelopeStage::Idle
    }
}

struct Voice {
    active: bool,
    note: u8,
    velocity: f32,
    base_frequency: f32,

    osc1: UnisonOscillator,
    osc2: UnisonOscillator,
    osc3: UnisonOscillator,

    filter: BiquadFilter,
    envelope: Envelope,
}

impl Voice {
    fn new(sample_rate: f32) -> Self {
        Self {
            active: false,
            note: 0,
            velocity: 0.0,
            base_frequency: 440.0,
            osc1: UnisonOscillator::new(8),
            osc2: UnisonOscillator::new(8),
            osc3: UnisonOscillator::new(8),
            filter: BiquadFilter::new(sample_rate),
            envelope: Envelope::new(sample_rate),
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        self.active = true;
        self.note = note;
        self.velocity = velocity;
        self.base_frequency = 440.0 * (2.0_f32).powf((note as f32 - 69.0) / 12.0);
        self.osc1.reset();
        self.osc2.reset();
        self.osc3.reset();
        self.filter.reset();
        self.envelope.note_on();
    }

    fn note_off(&mut self) {
        self.envelope.note_off();
    }

    fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.filter.set_sample_rate(sample_rate);
        self.envelope.set_sample_rate(sample_rate);
    }
}

#[derive(Params)]
pub struct SineParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[id = "waveform1"]
    pub waveform1: EnumParam<Waveform>,
    #[id = "freq1"]
    pub frequency1: FloatParam,
    #[id = "detune1"]
    pub detune1: FloatParam,
    #[id = "phase1"]
    pub phase1: FloatParam,
    #[id = "gain1"]
    pub gain1: FloatParam,
    #[id = "octave1"]
    pub octave1: IntParam,
    #[id = "unison_voices1"]
    pub unison_voices1: IntParam,
    #[id = "unison_detune1"]
    pub unison_detune1: FloatParam,
    #[id = "unison_blend1"]
    pub unison_blend1: FloatParam,
    #[id = "unison_volume1"]
    pub unison_volume1: FloatParam,

    #[id = "waveform2"]
    pub waveform2: EnumParam<Waveform>,
    #[id = "freq2"]
    pub frequency2: FloatParam,
    #[id = "detune2"]
    pub detune2: FloatParam,
    #[id = "phase2"]
    pub phase2: FloatParam,
    #[id = "gain2"]
    pub gain2: FloatParam,
    #[id = "octave2"]
    pub octave2: IntParam,
    #[id = "unison_voices2"]
    pub unison_voices2: IntParam,
    #[id = "unison_detune2"]
    pub unison_detune2: FloatParam,
    #[id = "unison_blend2"]
    pub unison_blend2: FloatParam,
    #[id = "unison_volume2"]
    pub unison_volume2: FloatParam,

    #[id = "waveform3"]
    pub waveform3: EnumParam<Waveform>,
    #[id = "freq3"]
    pub frequency3: FloatParam,
    #[id = "detune3"]
    pub detune3: FloatParam,
    #[id = "phase3"]
    pub phase3: FloatParam,
    #[id = "gain3"]
    pub gain3: FloatParam,
    #[id = "octave3"]
    pub octave3: IntParam,
    #[id = "unison_voices3"]
    pub unison_voices3: IntParam,
    #[id = "unison_detune3"]
    pub unison_detune3: FloatParam,
    #[id = "unison_blend3"]
    pub unison_blend3: FloatParam,
    #[id = "unison_volume3"]
    pub unison_volume3: FloatParam,

    #[id = "filter_mode"]
    pub filter_mode: EnumParam<FilterMode>,
    #[id = "filter_cutoff"]
    pub filter_cutoff: FloatParam,
    #[id = "filter_resonance"]
    pub filter_resonance: FloatParam,
    #[id = "filter_drive"]
    pub filter_drive: FloatParam,

    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "release"]
    pub release: FloatParam,
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            waveform1: EnumParam::new("Waveform 1", Waveform::default()),
            frequency1: FloatParam::new(
                "Frequency 1",
                440.0,
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

            detune1: FloatParam::new(
                "Detune 1",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit(" cents")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            phase1: FloatParam::new("Phase 1", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
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

            gain1: FloatParam::new(
                "Gain 1",
                util::db_to_gain(-6.0),
                FloatRange::Linear {
                    min: util::db_to_gain(-36.0),
                    max: util::db_to_gain(0.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            octave1: IntParam::new("Octave 1", 0, IntRange::Linear { min: -4, max: 4 }),

            unison_voices1: IntParam::new(
                "Unison Voices 1",
                1,
                IntRange::Linear { min: 1, max: 8 },
            )
            .with_unit(" voices"),
            unison_detune1: FloatParam::new(
                "Unison Detune 1",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 50.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit(" cents"),
            unison_blend1: FloatParam::new(
                "Unison Blend 1",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_percentage(1)),
            unison_volume1: FloatParam::new(
                "Unison Volume 1",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(formatters::v2s_f32_percentage(1)),

            waveform2: EnumParam::new("Waveform 2", Waveform::Sawtooth),
            frequency2: FloatParam::new(
                "Frequency 2",
                880.0,
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

            detune2: FloatParam::new(
                "Detune 2",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit(" cents"),
            phase2: FloatParam::new("Phase 2", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(50.0))
                .with_value_to_string(Arc::new(|value| format!("{:.0}°", value * 360.0))),
            gain2: FloatParam::new(
                "Gain 2",
                util::db_to_gain(-12.0),
                FloatRange::Linear {
                    min: util::db_to_gain(-36.0),
                    max: util::db_to_gain(0.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2)),

            octave2: IntParam::new("Octave 2", -1, IntRange::Linear { min: -4, max: 4 }),
            unison_voices2: IntParam::new(
                "Unison Voices 2",
                1,
                IntRange::Linear { min: 1, max: 8 },
            ),
            unison_detune2: FloatParam::new(
                "Unison Detune 2",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 50.0,
                },
            ),
            unison_blend2: FloatParam::new(
                "Unison Blend 2",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            unison_volume2: FloatParam::new(
                "Unison Volume 2",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            waveform3: EnumParam::new("Waveform 3", Waveform::Square),
            frequency3: FloatParam::new(
                "Frequency 3",
                220.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20_000.0,
                    factor: 0.5,
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2)),

            detune3: FloatParam::new(
                "Detune 3",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            ),
            phase3: FloatParam::new("Phase 3", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            gain3: FloatParam::new(
                "Gain 3",
                util::db_to_gain(-18.0),
                FloatRange::Linear {
                    min: util::db_to_gain(-36.0),
                    max: util::db_to_gain(0.0),
                },
            ),
            octave3: IntParam::new("Octave 3", 1, IntRange::Linear { min: -4, max: 4 }),
            unison_voices3: IntParam::new(
                "Unison Voices 3",
                1,
                IntRange::Linear { min: 1, max: 8 },
            ),
            unison_detune3: FloatParam::new(
                "Unison Detune 3",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 50.0,
                },
            ),
            unison_blend3: FloatParam::new(
                "Unison Blend 3",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            unison_volume3: FloatParam::new(
                "Unison Volume 3",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            filter_mode: EnumParam::new("Filter Mode", FilterMode::LowPass),
            filter_cutoff: FloatParam::new(
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

            filter_resonance: FloatParam::new(
                "Filter Resonance",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit("%")
            .with_value_to_string(formatters::v2s_f32_percentage(0)),

            filter_drive: FloatParam::new(
                "Filter Drive",
                1.0,
                FloatRange::Linear { min: 1.0, max: 5.0 },
            )
            .with_smoother(SmoothingStyle::Linear(50.0)),

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

pub struct SineSynth {
    params: Arc<SineParams>,
    sample_rate: f32,
    voices: Vec<Voice>,
}

impl Default for SineSynth {
    fn default() -> Self {
        let sample_rate = 44100.0;
        let mut voices = Vec::with_capacity(16);
        for _ in 0..16 {
            voices.push(Voice::new(sample_rate));
        }

        Self {
            params: Arc::new(SineParams::default()),
            sample_rate,
            voices,
        }
    }
}

impl Plugin for SineSynth {
    const NAME: &'static str = "Triple Oscillator Synth";
    const VENDOR: &'static str = "Your Name";
    const URL: &'static str = "www.your.website";
    const EMAIL: &'static str = "your@email.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        for voice in &mut self.voices {
            voice.set_sample_rate(self.sample_rate);
        }
        true
    }

    fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            voice.osc1.reset();
            voice.osc2.reset();
            voice.osc3.reset();
            voice.filter.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    if velocity > 0.0 {
                        
                        if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
                            voice.note_on(note, velocity);
                        } else {
                            
                            if let Some((oldest_idx, _)) = self
                                .voices
                                .iter()
                                .enumerate()
                                .min_by_key(|(_, v)| v.envelope.samples_elapsed)
                            {
                                self.voices[oldest_idx].note_on(note, velocity);
                            }
                        }
                    }
                }

                NoteEvent::NoteOff { note, .. } => {
                    for voice in &mut self.voices {
                        if voice.note == note && voice.active {
                            voice.note_off();
                            
                        }
                    }
                }

                NoteEvent::Choke { .. } => {
                    for voice in &mut self.voices {
                        voice.note_off();
                    }
                }
                _ => {}
            }
        }

        
        for (_frame_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let mut sample = 0.0;

            for voice in &mut self.voices {
                if !voice.is_active() {
                    continue;
                }

                
                voice
                    .osc1
                    .set_num_voices(self.params.unison_voices1.value() as usize);
                voice
                    .osc2
                    .set_num_voices(self.params.unison_voices2.value() as usize);
                voice
                    .osc3
                    .set_num_voices(self.params.unison_voices3.value() as usize);

                
                let mut voice_sample = 0.0;

                
                let freq1 = voice.base_frequency
                    * 2.0_f32.powf(self.params.octave1.value() as f32)
                    * (self.params.frequency1.smoothed.next() / 440.0)
                    * 2.0_f32.powf(self.params.detune1.smoothed.next() / 1200.0);

                let osc1_out = voice.osc1.process(
                    self.params.waveform1.value(),
                    freq1,
                    self.params.unison_detune1.smoothed.next(),
                    self.params.phase1.smoothed.next(),
                    self.params.unison_blend1.smoothed.next(),
                    self.params.unison_volume1.smoothed.next(),
                    self.sample_rate,
                ) * self.params.gain1.smoothed.next();

                
                let freq2 = voice.base_frequency
                    * 2.0_f32.powf(self.params.octave2.value() as f32)
                    * (self.params.frequency2.smoothed.next() / 440.0)
                    * 2.0_f32.powf(self.params.detune2.smoothed.next() / 1200.0);

                let osc2_out = voice.osc2.process(
                    self.params.waveform2.value(),
                    freq2,
                    self.params.unison_detune2.smoothed.next(),
                    self.params.phase2.smoothed.next(),
                    self.params.unison_blend2.smoothed.next(),
                    self.params.unison_volume2.smoothed.next(),
                    self.sample_rate,
                ) * self.params.gain2.smoothed.next();

                
                let freq3 = voice.base_frequency
                    * 2.0_f32.powf(self.params.octave3.value() as f32)
                    * (self.params.frequency3.smoothed.next() / 440.0)
                    * 2.0_f32.powf(self.params.detune3.smoothed.next() / 1200.0);

                let osc3_out = voice.osc3.process(
                    self.params.waveform3.value(),
                    freq3,
                    self.params.unison_detune3.smoothed.next(),
                    self.params.phase3.smoothed.next(),
                    self.params.unison_blend3.smoothed.next(),
                    self.params.unison_volume3.smoothed.next(),
                    self.sample_rate,
                ) * self.params.gain3.smoothed.next();

                voice_sample = osc1_out + osc2_out + osc3_out;

                
                voice.filter.set_coefficients(
                    self.params.filter_mode.value(),
                    self.params.filter_cutoff.smoothed.next(),
                    self.params.filter_resonance.smoothed.next(),
                );
                voice_sample = voice
                    .filter
                    .process(voice_sample, self.params.filter_drive.smoothed.next());

                
                
                let envelope_level = voice.envelope.process(
                    self.params.attack.smoothed.next().max(0.001), 
                    self.params.decay.smoothed.next().max(0.001),  
                    self.params.sustain.smoothed.next().clamp(0.0, 1.0), 
                    self.params.release.smoothed.next().max(0.001), 
                );

                voice_sample *= envelope_level * voice.velocity;
                sample += voice_sample;

                if !voice.envelope.is_active() {
                    voice.active = false;
                }
            }

            
            sample = sample.tanh() * 0.5;

            
            for output_sample in channel_samples {
                *output_sample = sample;
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for SineSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"TriOscSynth2025!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Synth];
}

impl ClapPlugin for SineSynth {
    const CLAP_ID: &'static str = "com.yourdomain.triple-osc-synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Triple oscillator wave synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument];
}

nih_export_clap!(SineSynth);
nih_export_vst3!(SineSynth);
