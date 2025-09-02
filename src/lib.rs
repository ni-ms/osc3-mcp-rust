use nih_plug::midi::{MidiConfig, NoteEvent};
use nih_plug::prelude::*;
use nih_plug_egui::{EguiState, create_egui_editor, egui, widgets};
use std::f32::consts::TAU;
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct SineSynth {
    params: Arc<SineParams>,
    phase: f32,
    sample_rate: f32,

    current_note: Option<u8>,
    current_freq: f32,
    gate: bool,
}

#[derive(Params)]
struct SineParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "freq"]
    frequency: FloatParam,

    #[id = "gain"]
    gain: FloatParam,

    // Optional: audible idle test tone level (set to 0.0 to disable)
    #[id = "idle_db"]
    idle_db: FloatParam,
}

impl Default for SineSynth {
    fn default() -> Self {
        Self {
            params: Arc::new(SineParams::default()),
            phase: 0.0,
            sample_rate: 44100.0,

            current_note: None,
            current_freq: 440.0,
            gate: false,
        }
    }
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(300, 200),
            frequency: FloatParam::new(
                "Frequency",
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
            gain: FloatParam::new(
                "Gain",
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
            idle_db: FloatParam::new(
                "Idle Tone",
                util::db_to_gain(-60.0), // -inf..-40 dB range, default -60 dB (very quiet)
                FloatRange::Linear {
                    min: 0.0,
                    max: util::db_to_gain(-40.0),
                },
            )
                .with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for SineSynth {
    const NAME: &'static str = "Simple Sine Synth";
    const VENDOR: &'static str = "Your Name";
    const URL: &'static str = "https://your.website";
    const EMAIL: &'static str = "your@email.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // Instrument: no main input, stereo output only (simplifies FL Studio routing)
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    // Enable MIDI note input
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.heading("Sine Wave Synth");
                    ui.add_space(10.0);
                    ui.label("Frequency");
                    ui.add(
                        widgets::ParamSlider::for_param(&params.frequency, setter)
                            .with_width(220.0),
                    );
                    ui.add_space(10.0);
                    ui.label("Gain");
                    ui.add(widgets::ParamSlider::for_param(&params.gain, setter).with_width(220.0));
                    ui.add_space(10.0);
                    ui.label("Idle Tone (for routing test)");
                    ui.add(
                        widgets::ParamSlider::for_param(&params.idle_db, setter).with_width(220.0),
                    );
                });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.current_note = None;
        self.current_freq = 440.0;
        self.gate = false;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Handle all incoming note/MIDI events
        while let Some(event) = context.next_event() {
            match event {
                // Treat velocity==0 as NoteOff (some hosts do this)
                NoteEvent::NoteOn { note, velocity, .. } => {
                    if velocity > 0.0 {
                        self.current_note = Some(note);
                        self.current_freq = 440.0 * (2.0_f32).powf((note as f32 - 69.0) / 12.0);
                        self.gate = true;
                    } else {
                        if Some(note) == self.current_note {
                            self.gate = false;
                            self.current_note = None;
                        }
                    }
                }
                NoteEvent::NoteOff { note, .. } => {
                    if Some(note) == self.current_note {
                        self.gate = false;
                        self.current_note = None;
                    }
                }
                NoteEvent::Choke { .. } => {
                    self.gate = false;
                    self.current_note = None;
                }
                _ => {}
            }
        }

        // Generate audio
        for (_frame_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let base_freq = self.params.frequency.smoothed.next();
            let gain = self.params.gain.smoothed.next();
            let idle = self.params.idle_db.smoothed.next(); // quiet test tone

            // Use MIDI note frequency if gated; otherwise emit an ultra-quiet test tone so routing is verifiable
            let freq = if self.gate {
                self.current_freq
            } else {
                base_freq
            };
            let phase_incr = (freq / self.sample_rate) * TAU;

            let sample = if self.gate {
                self.phase.sin() * gain
            } else {
                // test tone at a very low level; set Idle Tone to 0 dB in UI to disable completely
                self.phase.sin() * idle
            };

            for output_sample in channel_samples {
                *output_sample = sample;
            }

            self.phase += phase_incr;
            if self.phase >= TAU {
                self.phase -= TAU;
            }
        }

        ProcessStatus::Normal
    }
}

// VST3 metadata
impl Vst3Plugin for SineSynth {
    // Use a unique, stable 16-byte ID (keep this constant once released)
    const VST3_CLASS_ID: [u8; 16] = *b"SineSynthFL2025!"; // exactly 16 bytes

    // Keep this strictly a synth to ensure generator classification
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Synth];
}

// CLAP metadata
impl ClapPlugin for SineSynth {
    // Use reverse-DNS; keep stable once users have projects with it
    const CLAP_ID: &'static str = "com.yourdomain.simple-sine-synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Simple mono sine wave synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument];
}

nih_export_clap!(SineSynth);
