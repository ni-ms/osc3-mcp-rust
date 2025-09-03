use nih_plug::midi::{MidiConfig, NoteEvent};
use nih_plug::prelude::*;
use nih_plug_egui::{EguiState, create_egui_editor, egui, widgets};
use std::f32::consts::TAU;
use std::num::NonZeroU32;
use std::sync::Arc;

use nih_plug::params::EnumParam;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}
impl Default for Waveform {
    fn default() -> Self {
        Self::Sine
    }
}

#[derive(Params)]
struct SineParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "waveform"]
    waveform: EnumParam<Waveform>,

    #[id = "freq"]
    frequency: FloatParam,

    #[id = "gain"]
    gain: FloatParam,
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(700, 250),
            waveform: EnumParam::new("Waveform", Waveform::default()),
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

pub struct SineSynth {
    params: Arc<SineParams>,
    phase: f32,
    sample_rate: f32,
    current_note: Option<u8>,

    current_freq: f32,
    target_freq: f32,
    freq_smoother: SmoothedValue,

    gate: bool,
}

pub struct SmoothedValue {
    sample_rate: f32,
    smoothing_time_s: f32,
    current: f32,
    step: f32,
}

impl SmoothedValue {
    pub fn new(sample_rate: f32, smoothing_time_s: f32) -> Self {
        Self {
            sample_rate,
            smoothing_time_s,
            current: 0.0,
            step: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.step = 0.0;
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
        Self {
            params: Arc::new(SineParams::default()),
            phase: 0.0,
            sample_rate: 44100.0,
            current_note: None,
            current_freq: 440.0,
            target_freq: 440.0,
            freq_smoother: SmoothedValue::new(44100.0, 0.005),
            gate: false,
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
                        ui.label("Waveform");
                        egui::ComboBox::from_label("Waveform")
                            .selected_text(format!("{:?}", params.waveform.value()))
                            .show_ui(ui, |ui| {
                                for &variant in &[
                                    Waveform::Sine,
                                    Waveform::Square,
                                    Waveform::Triangle,
                                    Waveform::Sawtooth,
                                ] {
                                    if ui
                                        .selectable_label(
                                            params.waveform.value() == variant,
                                            format!("{:?}", variant),
                                        )
                                        .clicked()
                                    {
                                        setter.set_parameter(&params.waveform, variant);
                                    }
                                }
                            });

                        ui.add_space(20.0);

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
        self.freq_smoother.set_sample_rate(self.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.current_note = None;
        self.current_freq = 440.0;
        self.target_freq = 440.0;
        self.gate = false;
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
                        self.current_note = Some(note);
                        self.target_freq = 440.0 * (2.0_f32).powf((note as f32 - 69.0) / 12.0);
                        self.gate = true;
                    } else if Some(note) == self.current_note {
                        self.gate = false;
                        self.current_note = None;
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

        let waveform = self.params.waveform.value();

        for (_frame_idx, channel_samples) in buffer.iter_samples().enumerate() {
            let gain = self.params.gain.smoothed.next();

            // Smooth frequency update
            self.current_freq = self.freq_smoother.next(self.target_freq);

            let freq = if self.gate { self.current_freq } else { 0.0 };
            let phase_incr = (freq / self.sample_rate) * TAU;

            let sample = if self.gate {
                match waveform {
                    Waveform::Sine => self.phase.sin() * gain,
                    Waveform::Square => {
                        if self.phase < std::f32::consts::PI {
                            gain
                        } else {
                            -gain
                        }
                    }
                    Waveform::Triangle => {
                        ((2.0 * (self.phase / TAU) - 1.0).abs() * 2.0 - 1.0) * gain
                    }
                    Waveform::Sawtooth => ((self.phase / TAU) * 2.0 - 1.0) * gain,
                }
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

impl Vst3Plugin for SineSynth {
    const VST3_CLASS_ID: [u8; 16] = *b"SineSynthFL2025!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Synth];
}

impl ClapPlugin for SineSynth {
    const CLAP_ID: &'static str = "com.yourdomain.simple-sine-synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Simple mono wave synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Instrument];
}

nih_export_clap!(SineSynth);
