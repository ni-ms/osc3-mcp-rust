use super::envelope::Envelope;
use super::filter::BiquadFilter;
use super::oscillator::UnisonOscillator;
use crate::params::{OscillatorParams, SineParams};
use crate::{FilterMode, Waveform};

/// Per-oscillator parameter values for a single sample frame.
///
/// Smoothed parameters are read **once per sample** here and shared across all
/// voices. Reading them per-voice (as the old code did) advanced the smoothers
/// N times per sample for N active voices.
pub struct OscFrame {
    waveform: Waveform,
    /// `2^octave`, precomputed.
    octave_mult: f32,
    /// Frequency knob expressed as a ratio relative to 440 Hz.
    freq_ratio: f32,
    /// `2^(detune_cents / 1200)`, precomputed.
    detune_mult: f32,
    unison_detune: f32,
    phase: f32,
    blend: f32,
    volume: f32,
    gain: f32,
}

impl OscFrame {
    fn next(p: &OscillatorParams) -> Self {
        Self {
            waveform: p.waveform.value(),
            octave_mult: 2.0_f32.powf(p.octave.value() as f32),
            freq_ratio: p.frequency.smoothed.next() / 440.0,
            detune_mult: 2.0_f32.powf(p.detune.smoothed.next() / 1200.0),
            unison_detune: p.unison_detune.smoothed.next(),
            phase: p.phase.smoothed.next(),
            blend: p.unison_blend.smoothed.next(),
            volume: p.unison_volume.smoothed.next(),
            gain: p.gain.smoothed.next(),
        }
    }
}

/// A snapshot of every smoothed parameter value for one sample frame, built once
/// per sample and fed to every active voice.
pub struct FrameParams {
    osc: [OscFrame; 3],
    filter_mode: FilterMode,
    filter_cutoff: f32,
    filter_resonance: f32,
    filter_drive: f32,
    /// Filter-envelope depth in octaves (bipolar). `0` means the filter envelope
    /// has no effect on the cutoff.
    filter_env_amount: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    /// Filter-envelope ADSR, separate from the amp envelope above.
    filter_attack: f32,
    filter_decay: f32,
    filter_sustain: f32,
    filter_release: f32,
}

impl FrameParams {
    /// Advances every smoother exactly one step. Call once per output sample.
    pub fn next(p: &SineParams) -> Self {
        Self {
            osc: [
                OscFrame::next(&p.osc1),
                OscFrame::next(&p.osc2),
                OscFrame::next(&p.osc3),
            ],
            filter_mode: p.filter.mode.value(),
            filter_cutoff: p.filter.cutoff.smoothed.next(),
            filter_resonance: p.filter.resonance.smoothed.next(),
            filter_drive: p.filter.drive.smoothed.next(),
            filter_env_amount: p.filter.env_amount.smoothed.next(),
            attack: p.adsr.attack.smoothed.next().max(0.001),
            decay: p.adsr.decay.smoothed.next().max(0.001),
            sustain: p.adsr.sustain.smoothed.next().clamp(0.0, 1.0),
            release: p.adsr.release.smoothed.next().max(0.001),
            filter_attack: p.filter_env.attack.smoothed.next().max(0.001),
            filter_decay: p.filter_env.decay.smoothed.next().max(0.001),
            filter_sustain: p.filter_env.sustain.smoothed.next().clamp(0.0, 1.0),
            filter_release: p.filter_env.release.smoothed.next().max(0.001),
        }
    }
}

pub struct Voice {
    active: bool,
    note: u8,
    velocity: f32,
    base_frequency: f32,

    osc1: UnisonOscillator,
    osc2: UnisonOscillator,
    osc3: UnisonOscillator,

    filter: BiquadFilter,
    envelope: Envelope,
    /// Modulates the filter cutoff; runs in lockstep with `envelope` (same
    /// note-on/note-off), scaled by `FrameParams::filter_env_amount`.
    filter_env: Envelope,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
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
            filter_env: Envelope::new(sample_rate),
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.active = true;
        self.note = note;
        self.velocity = velocity;
        self.base_frequency = 440.0 * (2.0_f32).powf((note as f32 - 69.0) / 12.0);
        self.osc1.reset();
        self.osc2.reset();
        self.osc3.reset();
        self.filter.reset();
        self.envelope.note_on();
        self.filter_env.note_on();
    }

    pub fn note_off(&mut self) {
        self.envelope.note_off();
        self.filter_env.note_off();
    }

    /// Begins the release stage if this voice is playing the given note.
    pub fn release_if_matches(&mut self, note: u8) {
        if self.active && self.note == note {
            self.envelope.note_off();
            self.filter_env.note_off();
        }
    }

    /// Whether this slot is available for a new note.
    pub fn is_free(&self) -> bool {
        !self.active
    }

    /// Voice-stealing priority: the longer a voice has been playing, the more
    /// eligible it is to be stolen.
    pub fn age(&self) -> u32 {
        self.envelope.samples_elapsed()
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.filter.set_sample_rate(sample_rate);
        self.envelope.set_sample_rate(sample_rate);
        self.filter_env.set_sample_rate(sample_rate);
    }

    /// Clears oscillator/filter state without touching the envelope (used by
    /// `Plugin::reset`).
    pub fn reset(&mut self) {
        self.active = false;
        self.osc1.reset();
        self.osc2.reset();
        self.osc3.reset();
        self.filter.reset();
    }

    /// Sets the unison voice count for all three oscillators. This is a
    /// control-rate concern, so it runs once per process block rather than per
    /// sample.
    pub fn set_unison_voices(&mut self, counts: [usize; 3]) {
        self.osc1.set_num_voices(counts[0]);
        self.osc2.set_num_voices(counts[1]);
        self.osc3.set_num_voices(counts[2]);
    }

    /// Renders one sample from the shared per-frame parameter snapshot.
    pub fn render(&mut self, f: &FrameParams, sample_rate: f32) -> f32 {
        let base = self.base_frequency;
        let mut sample = render_osc(&mut self.osc1, &f.osc[0], base, sample_rate)
            + render_osc(&mut self.osc2, &f.osc[1], base, sample_rate)
            + render_osc(&mut self.osc3, &f.osc[2], base, sample_rate);

        // Advance the filter envelope in lockstep with the amp envelope and use
        // it to push the cutoff up/down by `env_amount` octaves. `2^0 == 1`, so
        // an amount of 0 leaves the cutoff exactly at the knob value. The filter
        // itself re-clamps the result to [20 Hz, Nyquist].
        let filter_env_level = self.filter_env.process(
            f.filter_attack,
            f.filter_decay,
            f.filter_sustain,
            f.filter_release,
        );
        let modulated_cutoff = f.filter_cutoff * 2.0_f32.powf(f.filter_env_amount * filter_env_level);

        self.filter
            .set_coefficients(f.filter_mode, modulated_cutoff, f.filter_resonance);
        sample = self.filter.process(sample, f.filter_drive);

        let envelope_level = self
            .envelope
            .process(f.attack, f.decay, f.sustain, f.release);

        if !self.envelope.is_active() {
            self.active = false;
        }

        sample * envelope_level * self.velocity
    }

    /// Whether the voice is still producing sound (envelope not idle).
    pub fn is_active(&self) -> bool {
        self.envelope.is_active()
    }
}

fn render_osc(osc: &mut UnisonOscillator, fr: &OscFrame, base_freq: f32, sample_rate: f32) -> f32 {
    let freq = base_freq * fr.octave_mult * fr.freq_ratio * fr.detune_mult;
    osc.process(
        fr.waveform,
        freq,
        fr.unison_detune,
        fr.phase,
        fr.blend,
        fr.volume,
        sample_rate,
    ) * fr.gain
}
