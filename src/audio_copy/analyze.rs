//! Local, offline "match" — turn a stem into a synth patch ([`PresetData`])
//! with FFT spectral analysis and amplitude-envelope tracking.
//!
//! The synth re-pitches per MIDI note (the osc `frequency` param is a 440-Hz
//! *ratio*, see `dsp/voice.rs::render_osc`), so absolute pitch is intentionally
//! NOT matched here — we copy timbre: waveform from harmonicity, filter cutoff
//! from spectral centroid, and ADSR from the amplitude envelope.

use rustfft::{FftPlanner, num_complex::Complex};

use crate::SineParams;
use crate::ai::preset::PresetData;

use super::separate::{Stem, StemKind};

/// Extracted descriptors used to choose patch parameters.
struct Analysis {
    /// Spectral centroid in Hz (brightness).
    centroid: f32,
    /// 0 = noise-like, 1 = pure tone (1 − spectral flatness).
    harmonicity: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

/// Build a synth patch that approximates `stem`. `base` provides a valid
/// starting snapshot (every field already in range) that we selectively override.
pub fn match_local(stem: &Stem, base: &SineParams) -> PresetData {
    let a = analyze(&stem.samples, stem.sample_rate, stem.kind);
    let wave = pick_waveform(a.harmonicity, stem.kind);
    let bass_register = a.centroid < 400.0 || stem.label.eq_ignore_ascii_case("bass");

    let mut p = PresetData::capture(base);

    // Three oscillators on the same waveform: osc1 lead, osc2 slightly detuned
    // for width, osc3 quiet sub/body. Pitch left at the 440 ratio (the MIDI note
    // sets absolute pitch).
    let w: String = wave.to_string();
    p.waveform1 = w.clone();
    p.frequency1 = 440.0;
    p.octave1 = 0;
    p.detune1 = 0.0;
    p.phase1 = 0.0;
    p.gain1 = 1.0;
    p.unison_voices1 = 1;

    p.waveform2 = w.clone();
    p.frequency2 = 440.0;
    p.octave2 = 0;
    p.detune2 = 7.0;
    p.gain2 = 0.3;
    p.unison_voices2 = 1;

    p.waveform3 = w;
    p.frequency3 = 440.0;
    p.octave3 = if bass_register { -1 } else { 0 };
    p.detune3 = -7.0;
    p.gain3 = 0.2;
    p.unison_voices3 = 1;

    // Filter: low-pass at the centroid (brightness), gentle resonance.
    p.filter_mode = "lowpass".to_string();
    p.filter_cutoff = a.centroid.clamp(80.0, 18_000.0);
    p.filter_resonance = 0.12;
    p.filter_drive = 1.0;
    p.filter_env_amount = 0.0;

    // Amplitude envelope from the analysis.
    p.attack = a.attack;
    p.decay = a.decay;
    p.sustain = a.sustain;
    p.release = a.release;

    p.name = format!("AudioCopy — {}", stem.label);
    p
}

fn pick_waveform(harmonicity: f32, kind: StemKind) -> &'static str {
    if kind == StemKind::Percussive {
        // Noisy/percussive: a rich waveform under a moving filter reads best.
        return "sawtooth";
    }
    if harmonicity > 0.85 {
        "sine"
    } else if harmonicity > 0.6 {
        "triangle"
    } else {
        "sawtooth"
    }
}

fn analyze(samples: &[f32], sample_rate: u32, kind: StemKind) -> Analysis {
    let sr = sample_rate.max(1) as f32;
    let (centroid, harmonicity) = spectral(samples, sr);
    let (attack, decay, mut sustain, release) = envelope(samples, sr);

    // Percussive elements don't sustain.
    if kind == StemKind::Percussive {
        sustain = sustain.min(0.1);
    }

    Analysis {
        centroid,
        harmonicity,
        attack,
        decay,
        sustain,
        release,
    }
}

/// Spectral centroid (Hz) and harmonicity (1 − flatness) from a windowed FFT
/// taken around the loudest region of the signal.
fn spectral(samples: &[f32], sr: f32) -> (f32, f32) {
    const N: usize = 8192;
    if samples.len() < 64 {
        return (1000.0, 0.5);
    }

    // Center the analysis window on the peak-amplitude sample.
    let peak_idx = samples
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.abs()
                .partial_cmp(&b.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    let n = N.min(samples.len());
    let start = peak_idx.saturating_sub(n / 2).min(samples.len() - n);

    let mut buf: Vec<Complex<f32>> = (0..n)
        .map(|i| {
            let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos();
            Complex::new(samples[start + i] * w, 0.0)
        })
        .collect();

    let mut planner = FftPlanner::<f32>::new();
    planner.plan_fft_forward(n).process(&mut buf);

    let bin_hz = sr / n as f32;
    let mut weighted = 0.0f32;
    let mut total = 0.0f32;
    let mut log_sum = 0.0f32;
    let mut count = 0u32;
    for (k, c) in buf.iter().enumerate().take(n / 2).skip(1) {
        let m = c.norm();
        weighted += k as f32 * bin_hz * m;
        total += m;
        log_sum += (m + 1e-9).ln();
        count += 1;
    }

    let centroid = if total > 1e-9 {
        weighted / total
    } else {
        1000.0
    };

    // Spectral flatness = geometric mean / arithmetic mean of the magnitudes.
    let flatness = if total > 1e-9 && count > 0 {
        let geo = (log_sum / count as f32).exp();
        let arith = total / count as f32;
        (geo / arith).clamp(0.0, 1.0)
    } else {
        0.5
    };

    (centroid, 1.0 - flatness)
}

/// Track the amplitude envelope and derive ADSR (seconds, seconds, 0..1, seconds).
fn envelope(samples: &[f32], sr: f32) -> (f32, f32, f32, f32) {
    const WIN: usize = 256;
    const HOP: usize = 128;

    if samples.len() < WIN * 2 {
        return (0.005, 0.1, 0.5, 0.2);
    }

    // Short-window RMS envelope.
    let mut env = Vec::new();
    let mut i = 0;
    while i + WIN <= samples.len() {
        let r = (samples[i..i + WIN].iter().map(|s| s * s).sum::<f32>() / WIN as f32).sqrt();
        env.push(r);
        i += HOP;
    }

    let tstep = HOP as f32 / sr;
    let (peak_idx, peak) = env
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, v)| (i, *v))
        .unwrap_or((0, 0.0));

    if peak < 1e-6 {
        return (0.005, 0.1, 0.5, 0.2);
    }

    // Onset: first crossing of 15% of the peak before the peak.
    let onset_idx = (0..=peak_idx)
        .find(|&j| env[j] > 0.15 * peak)
        .unwrap_or(0);
    let attack = ((peak_idx - onset_idx) as f32 * tstep).clamp(0.001, 5.0);

    // Tail: last index above 10% of the peak.
    let tail_idx = (0..env.len())
        .rev()
        .find(|&j| env[j] > 0.10 * peak)
        .unwrap_or(env.len().saturating_sub(1));

    // Sustain level: envelope at the midpoint of the held portion, vs the peak.
    let mid = (peak_idx + tail_idx) / 2;
    let sustain = (env[mid] / peak).clamp(0.0, 1.0);

    // Decay: time from the peak to where the envelope first falls to the sustain
    // level (a little above it, to land on the knee).
    let knee = (sustain * 1.05 * peak).max(0.1 * peak);
    let decay_idx = (peak_idx..env.len())
        .find(|&j| env[j] <= knee)
        .unwrap_or(tail_idx);
    let decay = ((decay_idx.saturating_sub(peak_idx)) as f32 * tstep).clamp(0.005, 3.0);

    // Release: from the tail to the end of the signal.
    let release =
        (((env.len().saturating_sub(tail_idx)) as f32 * tstep) + 0.05).clamp(0.01, 5.0);

    (attack, decay, sustain, release)
}
