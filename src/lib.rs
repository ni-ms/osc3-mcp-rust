use nih_plug::midi::{MidiConfig, NoteEvent};
use nih_plug::prelude::*;

use std::f32::consts::TAU;
use std::num::NonZeroU32;
use std::sync::Arc;
use vizia_plug::ViziaState;

mod editor;
mod knob;
mod tab_switcher;

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

struct Voice {
    active: bool,
    note: u8,
    phases: [f32; 3],
    freq_smoothers: [SmoothedValue; 3],
    target_freqs: [f32; 3],
    current_freqs: [f32; 3],
}

impl Voice {
    fn new(sample_rate: f32) -> Self {
        Self {
            active: false,
            note: 0,
            phases: [0.0; 3],
            freq_smoothers: [
                SmoothedValue::new(sample_rate, 0.005),
                SmoothedValue::new(sample_rate, 0.005),
                SmoothedValue::new(sample_rate, 0.005),
            ],
            target_freqs: [0.0; 3],
            current_freqs: [0.0; 3],
        }
    }

    fn note_on(&mut self, note: u8, base_freq: f32) {
        self.active = true;
        self.note = note;
        for i in 0..3 {
            self.target_freqs[i] = base_freq;
            self.current_freqs[i] = base_freq;
            self.freq_smoothers[i].reset(base_freq);
        }
    }

    fn note_off(&mut self, note: u8) {
        if self.note == note {
            self.active = false;
        }
    }
}

#[derive(Params)]
pub struct SineParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    // Oscillator 1
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

    // Oscillator 2
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

    // Oscillator 3
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

    #[id = "octave1"]
    pub octave1: IntParam,

    #[id = "octave2"]
    pub octave2: IntParam,

    #[id = "octave3"]
    pub octave3: IntParam,
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            // Oscillator 1
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

            // Oscillator 2
            waveform2: EnumParam::new("Waveform 2", Waveform::default()),
            frequency2: FloatParam::new(
                "Frequency 2",
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
            detune2: FloatParam::new(
                "Detune 2",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit(" cents")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            phase2: FloatParam::new("Phase 2", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
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
            gain2: FloatParam::new(
                "Gain 2",
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

            // Oscillator 3
            waveform3: EnumParam::new("Waveform 3", Waveform::default()),
            frequency3: FloatParam::new(
                "Frequency 3",
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
            detune3: FloatParam::new(
                "Detune 3",
                0.0,
                FloatRange::Linear {
                    min: -100.0,
                    max: 100.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_unit(" cents")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            phase3: FloatParam::new("Phase 3", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
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
            gain3: FloatParam::new(
                "Gain 3",
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
            octave1: IntParam::new(
                "Octave 1",
                0, // Default to 0 (no octave shift)
                IntRange::Linear { min: -4, max: 4 },
            ),

            octave2: IntParam::new("Octave 2", 0, IntRange::Linear { min: -4, max: 4 }),

            octave3: IntParam::new("Octave 3", 0, IntRange::Linear { min: -4, max: 4 }),
        }
    }
}

pub struct SineSynth {
    params: Arc<SineParams>,
    sample_rate: f32,
    voices: Vec<Voice>,
}

pub struct SmoothedValue {
    sample_rate: f32,
    smoothing_time_s: f32,
    current: f32,
}

impl SmoothedValue {
    pub fn new(sample_rate: f32, smoothing_time_s: f32) -> Self {
        Self {
            sample_rate,
            smoothing_time_s,
            current: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn reset(&mut self, value: f32) {
        self.current = value;
    }

    pub fn next(&mut self, target: f32) -> f32 {
        let total_samples = (self.smoothing_time_s * self.sample_rate).max(1.0);
        let step = (target - self.current) / total_samples;
        self.current += step;
        self.current
    }
}

impl Default for SineSynth {
    fn default() -> Self {
        let sample_rate = 44100.0;
        let mut voices = Vec::with_capacity(8);
        for _ in 0..8 {
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
    const URL: &'static str = "https://your.website";
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
            for smoother in &mut voice.freq_smoothers {
                smoother.set_sample_rate(self.sample_rate);
            }
        }
        true
    }

    fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
            voice.phases = [0.0; 3];
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
                        let base_freq = 440.0 * (2.0_f32).powf((note as f32 - 69.0) / 12.0);
                        if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
                            voice.note_on(note, base_freq);
                        } else {
                            self.voices[0].note_on(note, base_freq);
                        }
                    } else {
                        for voice in &mut self.voices {
                            voice.note_off(note);
                        }
                    }
                }
                NoteEvent::NoteOff { note, .. } => {
                    for voice in &mut self.voices {
                        voice.note_off(note);
                    }
                }
                NoteEvent::Choke { .. } => {
                    for voice in &mut self.voices {
                        voice.active = false;
                    }
                }
                _ => {}
            }
        }

        let waveform_params = [
            self.params.waveform1.value(),
            self.params.waveform2.value(),
            self.params.waveform3.value(),
        ];

        for (_frame_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let gains = [
                self.params.gain1.smoothed.next(),
                self.params.gain2.smoothed.next(),
                self.params.gain3.smoothed.next(),
            ];

            // Get detune values (in cents)
            let detunes = [
                self.params.detune1.smoothed.next(),
                self.params.detune2.smoothed.next(),
                self.params.detune3.smoothed.next(),
            ];

            // Get phase offsets (0-1, representing 0-360°)
            let phase_offsets = [
                self.params.phase1.smoothed.next() * TAU,
                self.params.phase2.smoothed.next() * TAU,
                self.params.phase3.smoothed.next() * TAU,
            ];

            let mut sample = 0.0;
            let mut active_voice_count = 0;

            for voice in &mut self.voices {
                if voice.active {
                    active_voice_count += 1;

                    for i in 0..3 {
                        let freq_multiplier = match i {
                            0 => self.params.frequency1.smoothed.next() / 440.0,
                            1 => self.params.frequency2.smoothed.next() / 440.0,
                            2 => self.params.frequency3.smoothed.next() / 440.0,
                            _ => 1.0,
                        };

                        // Apply detune: convert cents to frequency ratio (100 cents = 1 semitone)
                        let detune_multiplier = 2.0_f32.powf(detunes[i] / 1200.0);

                        let target_freq =
                            voice.target_freqs[i] * freq_multiplier * detune_multiplier;
                        voice.current_freqs[i] = voice.freq_smoothers[i].next(target_freq);
                    }

                    let mut voice_sample = 0.0;
                    for i in 0..3 {
                        let phase_incr = (voice.current_freqs[i] / self.sample_rate) * TAU;

                        // Apply phase offset
                        let current_phase = voice.phases[i] + phase_offsets[i];

                        let osc_sample = match waveform_params[i] {
                            Waveform::Sine => current_phase.sin(),
                            Waveform::Square => {
                                if current_phase % TAU < std::f32::consts::PI {
                                    1.0
                                } else {
                                    -1.0
                                }
                            }
                            Waveform::Triangle => {
                                (2.0 * ((current_phase % TAU) / TAU) - 1.0).abs() * 2.0 - 1.0
                            }
                            Waveform::Sawtooth => ((current_phase % TAU) / TAU) * 2.0 - 1.0,
                        };

                        voice_sample += osc_sample * gains[i];
                        voice.phases[i] += phase_incr;
                        if voice.phases[i] >= TAU {
                            voice.phases[i] -= TAU;
                        }
                    }
                    sample += voice_sample;
                }
            }

            let norm_factor = if active_voice_count == 0 {
                1.0
            } else {
                active_voice_count as f32
            };

            for output_sample in channel_samples {
                *output_sample = sample / norm_factor;
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
