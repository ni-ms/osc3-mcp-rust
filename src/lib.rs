use nih_plug::midi::{MidiConfig, NoteEvent};
use nih_plug::prelude::*;

use std::num::NonZeroU32;
use std::sync::Arc;

mod ai;
mod dsp;
mod editor;
mod knob;
mod params;
mod tab_switcher;

pub use params::{AdsrParams, FilterMode, FilterParams, OscillatorParams, SineParams, Waveform};

use dsp::{FrameParams, Voice};

/// Number of polyphonic voices in the pool.
const NUM_VOICES: usize = 16;

pub struct SineSynth {
    params: Arc<SineParams>,
    sample_rate: f32,
    voices: Vec<Voice>,
}

impl Default for SineSynth {
    fn default() -> Self {
        let sample_rate = 44100.0;
        let mut voices = Vec::with_capacity(NUM_VOICES);
        for _ in 0..NUM_VOICES {
            voices.push(Voice::new(sample_rate));
        }

        Self {
            params: Arc::new(SineParams::default()),
            sample_rate,
            voices,
        }
    }
}

impl SineSynth {
    /// Pushes the current unison voice counts to every voice. Control-rate, so
    /// this runs once per process block rather than per sample.
    fn sync_unison_voice_counts(&mut self) {
        let counts = [
            self.params.osc1.unison_voices.value() as usize,
            self.params.osc2.unison_voices.value() as usize,
            self.params.osc3.unison_voices.value() as usize,
        ];
        for voice in &mut self.voices {
            voice.set_unison_voices(counts);
        }
    }

    fn handle_note_event(&mut self, event: NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. } => {
                if velocity > 0.0 {
                    if let Some(voice) = self.voices.iter_mut().find(|v| v.is_free()) {
                        voice.note_on(note, velocity);
                    } else if let Some((oldest_idx, _)) =
                        self.voices.iter().enumerate().min_by_key(|(_, v)| v.age())
                    {
                        self.voices[oldest_idx].note_on(note, velocity);
                    }
                }
            }
            NoteEvent::NoteOff { note, .. } => {
                for voice in &mut self.voices {
                    voice.release_if_matches(note);
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
            voice.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        while let Some(event) = context.next_event() {
            self.handle_note_event(event);
        }

        self.sync_unison_voice_counts();

        for channel_samples in buffer.iter_samples() {
            // Advance every smoother exactly once for this sample, then share
            // the snapshot across all voices.
            let frame = FrameParams::next(&self.params);

            let mut sample = 0.0;
            for voice in self.voices.iter_mut().filter(|v| v.is_active()) {
                sample += voice.render(&frame, self.sample_rate);
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
