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
            editor_state: EguiState::from_size(700, 250),
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
        }
    }
}

impl Plugin for SineSynth {
    const NAME: &'static str = "Simple Sine Synth";
    const VENDOR: &'static str = "Your Name";
    const URL: &'static str = "https://your.website";
    const EMAIL: &'static str = "your@email.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

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
                    ui.add_space(20.0);

                    let available_width = ui.available_width();

                    ui.vertical(|ui| {
                        ui.label("Frequency");
                        ui.add(
                            widgets::ParamSlider::for_param(&params.frequency, setter)
                                .with_width(available_width),
                        );

                        ui.add_space(20.0);

                        ui.label("Gain");
                        ui.add(
                            widgets::ParamSlider::for_param(&params.gain, setter)
                                .with_width(available_width),
                        );
                    });
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

        // Generate audio only when a note gate is open; else output silence
        for (_frame_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let gain = self.params.gain.smoothed.next();

            let freq = if self.gate { self.current_freq } else { 0.0 };

            let phase_incr = (freq / self.sample_rate) * TAU;

            let sample = if self.gate {
                self.phase.sin() * gain
            } else {
                0.0
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

// CLAP metadata
impl ClapPlugin for SineSynth {
    const CLAP_ID: &'static str = "com.yourdomain.simple-sine-synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Simple mono sine wave synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument];
}

nih_export_clap!(SineSynth);
