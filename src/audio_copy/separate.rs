//! Source separation into auditionable, heuristically-labeled stems.
//!
//! The shipping engine is [`DspSeparator`]: a pure-Rust STFT pipeline that does
//! harmonic/percussive source separation (median-filtered soft masks, after
//! Fitzgerald 2010) and then frequency-band masking, producing a small fixed set
//! of labeled stems. [`MlSeparator`] is a stub behind the same [`Separator`]
//! trait so an ML backend (e.g. Demucs via ONNX) can drop in later.
//!
//! All of this allocates heavily and runs off the audio thread.

use std::sync::Arc;

use rustfft::{FftPlanner, num_complex::Complex};

use super::decode::Clip;

/// STFT frame size. 2048 @ 44.1 kHz ≈ 21.5 Hz bins — enough low-end resolution to
/// separate bass from kick.
const FRAME: usize = 2048;
/// Hop between frames (75% overlap → COLA-friendly with a Hann window).
const HOP: usize = 512;
/// Half-width of the median filter (window = 2*HALF+1 = 17).
const MEDIAN_HALF: usize = 8;

/// Whether a stem is the unprocessed mix, a tonal band, or a percussive band.
/// The local matcher uses this to pick a sensible starting patch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StemKind {
    Full,
    Harmonic,
    Percussive,
}

/// One isolated element of the clip.
#[derive(Clone)]
pub struct Stem {
    pub label: String,
    pub samples: Arc<Vec<f32>>,
    pub sample_rate: u32,
    pub kind: StemKind,
}

/// A separation backend.
pub trait Separator {
    fn separate(&self, clip: &Clip) -> Result<Vec<Stem>, String>;
}

/// Local DSP separation (default, ships now).
pub struct DspSeparator;

impl Separator for DspSeparator {
    fn separate(&self, clip: &Clip) -> Result<Vec<Stem>, String> {
        dsp_separate(clip)
    }
}

/// Placeholder ML backend. Not wired yet — returns an error so the UI can fall
/// back to DSP and tell the user. Future: `ort` + a Demucs-class model.
pub struct MlSeparator;

impl Separator for MlSeparator {
    fn separate(&self, _clip: &Clip) -> Result<Vec<Stem>, String> {
        Err("ML separation isn't available in this build yet.".to_string())
    }
}

/// Which engine the UI selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Dsp,
    Ml,
}

/// Run the chosen engine, falling back to DSP if the ML stub is unavailable. The
/// optional `String` is a user-facing note (e.g. the fallback reason).
pub fn separate(engine: Engine, clip: &Clip) -> Result<(Vec<Stem>, Option<String>), String> {
    match engine {
        Engine::Dsp => Ok((DspSeparator.separate(clip)?, None)),
        Engine::Ml => match MlSeparator.separate(clip) {
            Ok(stems) => Ok((stems, None)),
            Err(reason) => {
                let stems = DspSeparator.separate(clip)?;
                Ok((stems, Some(format!("{reason} Using DSP instead."))))
            }
        },
    }
}

// --- DSP implementation ------------------------------------------------------

fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos())
        .collect()
}

/// In-place median of a small slice via quickselect.
fn median(v: &mut [f32]) -> f32 {
    let mid = v.len() / 2;
    v.select_nth_unstable_by(mid, |a, b| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    });
    v[mid]
}

fn rms(sig: &[f32]) -> f32 {
    if sig.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = sig.iter().map(|s| s * s).sum();
    (sum_sq / sig.len() as f32).sqrt()
}

fn dsp_separate(clip: &Clip) -> Result<Vec<Stem>, String> {
    let sr = clip.sample_rate;
    let signal = clip.samples.as_slice();
    let n = signal.len();
    if n < FRAME {
        return Err("clip is too short to separate (need ≥ ~50 ms)".to_string());
    }

    let half = FRAME / 2;
    let nbins = half + 1; // 0..=Nyquist
    let num_frames = n.div_ceil(HOP);
    let window = hann(FRAME);

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME);
    let ifft = planner.plan_fft_inverse(FRAME);

    // --- Forward STFT: store complex frames + half-spectrum magnitude. ---
    let zero = Complex::new(0.0f32, 0.0f32);
    let mut frames = vec![zero; num_frames * FRAME];
    let mut mag = vec![0.0f32; num_frames * nbins];
    let mut scratch = vec![zero; FRAME];

    for t in 0..num_frames {
        let start = t * HOP;
        for (i, (slot, &w)) in scratch.iter_mut().zip(window.iter()).enumerate() {
            let idx = start + i;
            let s = if idx < n { signal[idx] } else { 0.0 };
            *slot = Complex::new(s * w, 0.0);
        }
        fft.process(&mut scratch);
        let fbase = t * FRAME;
        frames[fbase..fbase + FRAME].copy_from_slice(&scratch);
        let mbase = t * nbins;
        for (slot, c) in mag[mbase..mbase + nbins].iter_mut().zip(scratch.iter()) {
            *slot = c.norm();
        }
    }

    // --- Median-filtered harmonic (across time) & percussive (across freq) masks. ---
    let mut hmask = vec![0.0f32; num_frames * nbins];
    let mut pmask = vec![0.0f32; num_frames * nbins];
    let mut med = Vec::with_capacity(2 * MEDIAN_HALF + 1);

    // Harmonic: median along the time axis for each frequency bin.
    for k in 0..nbins {
        for t in 0..num_frames {
            med.clear();
            for d in 0..(2 * MEDIAN_HALF + 1) {
                let tt = (t as isize + d as isize - MEDIAN_HALF as isize)
                    .clamp(0, num_frames as isize - 1) as usize;
                med.push(mag[tt * nbins + k]);
            }
            hmask[t * nbins + k] = median(&mut med);
        }
    }

    // Percussive: median along the frequency axis for each frame.
    for t in 0..num_frames {
        let mbase = t * nbins;
        for k in 0..nbins {
            med.clear();
            for d in 0..(2 * MEDIAN_HALF + 1) {
                let kk = (k as isize + d as isize - MEDIAN_HALF as isize)
                    .clamp(0, nbins as isize - 1) as usize;
                med.push(mag[mbase + kk]);
            }
            pmask[mbase + k] = median(&mut med);
        }
    }

    // Soft (power) Wiener-style masks: h²/(h²+p²), p²/(h²+p²).
    for i in 0..num_frames * nbins {
        let h = hmask[i];
        let p = pmask[i];
        let denom = h * h + p * p + 1e-9;
        hmask[i] = h * h / denom;
        pmask[i] = p * p / denom;
    }

    // Overlap-add normalization (sum of window² at each output sample) — shared
    // by every reconstructed stem.
    let out_len = n;
    let mut norm = vec![0.0f32; out_len];
    for t in 0..num_frames {
        let start = t * HOP;
        for (i, &w) in window.iter().enumerate() {
            let idx = start + i;
            if idx < out_len {
                norm[idx] += w * w;
            }
        }
    }

    let bin_hz = sr as f32 / FRAME as f32;
    let inv_scale = 1.0 / FRAME as f32;

    // Reconstruct one stem = (harmonic|percussive mask) × [lo, hi) band mask.
    let mut recon_buf = vec![zero; FRAME];
    let mut reconstruct = |use_harm: bool, lo: f32, hi: f32| -> Vec<f32> {
        let mut out = vec![0.0f32; out_len];
        for t in 0..num_frames {
            let fbase = t * FRAME;
            let mbase = t * nbins;
            for kk in 0..FRAME {
                // Mirror the upper half onto the half-spectrum mask so the
                // masked spectrum stays conjugate-symmetric (→ real iFFT).
                let k = if kk <= half { kk } else { FRAME - kk };
                let base = if use_harm { hmask[mbase + k] } else { pmask[mbase + k] };
                let freq = k as f32 * bin_hz;
                let bandm = if freq >= lo && freq < hi { 1.0 } else { 0.0 };
                recon_buf[kk] = frames[fbase + kk] * (base * bandm);
            }
            ifft.process(&mut recon_buf);
            let start = t * HOP;
            for (i, &w) in window.iter().enumerate() {
                let idx = start + i;
                if idx < out_len {
                    out[idx] += recon_buf[i].re * inv_scale * w;
                }
            }
        }
        for (o, nrm) in out.iter_mut().zip(norm.iter()) {
            if *nrm > 1e-6 {
                *o /= *nrm;
            }
        }
        out
    };

    let nyquist = sr as f32 / 2.0;
    let full_rms = rms(signal).max(1e-6);

    // (label, harmonic?, lo Hz, hi Hz, kind)
    let defs: [(&str, bool, f32, f32, StemKind); 5] = [
        ("Bass", true, 0.0, 250.0, StemKind::Harmonic),
        ("Keys / pluck", true, 250.0, 2000.0, StemKind::Harmonic),
        ("Lead / bright", true, 2000.0, nyquist, StemKind::Harmonic),
        ("Kick / low perc", false, 0.0, 1500.0, StemKind::Percussive),
        ("Hi-hats / cymbals", false, 1500.0, nyquist, StemKind::Percussive),
    ];

    // Always offer the unprocessed mix first.
    let mut stems = vec![Stem {
        label: "Full mix".to_string(),
        samples: clip.samples.clone(),
        sample_rate: sr,
        kind: StemKind::Full,
    }];

    for (label, harm, lo, hi, kind) in defs {
        let sig = reconstruct(harm, lo, hi);
        // Drop near-silent bands so the list isn't cluttered with empty stems.
        if rms(&sig) > 0.04 * full_rms {
            stems.push(Stem {
                label: label.to_string(),
                samples: Arc::new(sig),
                sample_rate: sr,
                kind,
            });
        }
    }

    Ok(stems)
}
