use crate::Waveform;
use std::f32::consts::TAU;

#[derive(Clone)]
pub(crate) struct OscillatorVoice {
    phase: f32,
    detune_offset: f32,
}

#[derive(Clone)]
pub(crate) struct UnisonOscillator {
    voices: Vec<OscillatorVoice>,
    num_voices: usize,
}

impl UnisonOscillator {
    pub(crate) fn new(max_voices: usize) -> Self {
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

    pub(crate) fn set_num_voices(&mut self, num_voices: usize) {
        let num_voices = num_voices.min(self.voices.len()).max(1);
        if num_voices == self.num_voices {
            return;
        }
        self.num_voices = num_voices;

        for (i, voice) in self.voices.iter_mut().enumerate() {
            voice.detune_offset = if self.num_voices == 1 {
                0.0
            } else {
                (i as f32 - (self.num_voices - 1) as f32 / 2.0)
                    / ((self.num_voices - 1) as f32 / 2.0)
            };
        }
    }

    pub(crate) fn process(
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

    pub(crate) fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.phase = 0.0;
        }
    }
}
