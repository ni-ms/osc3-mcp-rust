use crate::FilterMode;

#[derive(Clone)]
pub(crate) struct BiquadFilter {
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
    pub(crate) fn new(sample_rate: f32) -> Self {
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

    pub(crate) fn set_coefficients(&mut self, mode: FilterMode, cutoff: f32, resonance: f32) {
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

    pub(crate) fn process(&mut self, input: f32, drive: f32) -> f32 {
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

    pub(crate) fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    pub(crate) fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.reset();
    }
}
