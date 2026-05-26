#[derive(Clone, Debug, PartialEq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone)]
pub(crate) struct Envelope {
    stage: EnvelopeStage,
    current_level: f32,
    sample_rate: f32,
    samples_elapsed: u32,
    release_start_level: f32,
}

impl Envelope {
    pub(crate) fn new(sample_rate: f32) -> Self {
        Self {
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
            sample_rate,
            samples_elapsed: 0,
            release_start_level: 0.0,
        }
    }

    pub(crate) fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub(crate) fn note_on(&mut self) {
        self.stage = EnvelopeStage::Attack;
        self.samples_elapsed = 0;
    }

    pub(crate) fn note_off(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.release_start_level = self.current_level;
            self.stage = EnvelopeStage::Release;
            self.samples_elapsed = 0;
        }
    }

    pub(crate) fn process(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
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

    pub(crate) fn is_active(&self) -> bool {
        self.stage != EnvelopeStage::Idle
    }

    /// How long the current note has been in its stage, used for voice-stealing
    /// priority (oldest voice wins).
    pub(crate) fn samples_elapsed(&self) -> u32 {
        self.samples_elapsed
    }
}
