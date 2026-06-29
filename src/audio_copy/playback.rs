//! Real-time-safe preview player for auditioning uploaded clips and isolated
//! stems through the synth's output.
//!
//! The audio thread only ever *reads* the currently-published buffer; the UI
//! thread publishes a new stem with a lock-free [`ArcSwapOption`] store. Reading
//! is wait-free and allocation-free, so [`PreviewPlayer::mix_into`] is safe to
//! call from `SineSynth::process` under `assert_process_allocs`.

use arc_swap::ArcSwapOption;
use nih_plug::buffer::Buffer;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Fixed audition gain. Preview is summed on top of the (already soft-clipped)
/// synth output, so keep some headroom.
const PREVIEW_GAIN: f32 = 0.9;

/// Shared, lock-free preview player. One instance is owned by [`crate::SineSynth`]
/// (read on the audio thread) and cloned into the Audio Copy panel (written from
/// the UI thread).
pub struct PreviewPlayer {
    /// The mono buffer currently loaded for playback, published by the UI thread.
    buffer: ArcSwapOption<Vec<f32>>,
    /// Sample rate of `buffer`, used to resample to the host rate on playback.
    sample_rate: AtomicU32,
    /// Output-frame cursor since playback started.
    position: AtomicUsize,
    /// Whether playback is active.
    playing: AtomicBool,
}

impl Default for PreviewPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewPlayer {
    pub fn new() -> Self {
        Self {
            buffer: ArcSwapOption::const_empty(),
            sample_rate: AtomicU32::new(44_100),
            position: AtomicUsize::new(0),
            playing: AtomicBool::new(false),
        }
    }

    /// Publish a stem/clip for playback (UI thread). Allocating the `Arc` is fine
    /// here; the audio thread only swaps in the pointer.
    pub fn set_buffer(&self, samples: Arc<Vec<f32>>, sample_rate: u32) {
        self.buffer.store(Some(samples));
        self.sample_rate.store(sample_rate.max(1), Ordering::Relaxed);
        self.position.store(0, Ordering::Relaxed);
    }

    /// Start playback from the beginning (UI thread).
    pub fn play(&self) {
        self.position.store(0, Ordering::Relaxed);
        self.playing.store(true, Ordering::Relaxed);
    }

    /// Stop playback (UI thread).
    pub fn stop(&self) {
        self.playing.store(false, Ordering::Relaxed);
    }

    /// Mix the preview into `buffer` (audio thread). Wait-free and
    /// allocation-free: a guarded load of the `Arc`, bounds-checked reads, and
    /// relaxed atomic cursor updates. Nearest-sample resampling is sufficient for
    /// an audition.
    pub fn mix_into(&self, buffer: &mut Buffer, host_sr: f32) {
        if !self.playing.load(Ordering::Relaxed) {
            return;
        }

        let guard = self.buffer.load();
        let Some(samples) = &*guard else {
            self.playing.store(false, Ordering::Relaxed);
            return;
        };
        let len = samples.len();
        if len == 0 {
            self.playing.store(false, Ordering::Relaxed);
            return;
        }

        let stem_sr = self.sample_rate.load(Ordering::Relaxed) as f32;
        let step = stem_sr / host_sr.max(1.0);
        let mut frame = self.position.load(Ordering::Relaxed);

        for channel_samples in buffer.iter_samples() {
            let src = (frame as f32 * step) as usize;
            if src >= len {
                // Reached the end — stop and rewind for the next press.
                self.playing.store(false, Ordering::Relaxed);
                self.position.store(0, Ordering::Relaxed);
                return;
            }
            let s = samples[src] * PREVIEW_GAIN;
            for out in channel_samples {
                *out += s;
            }
            frame += 1;
        }

        self.position.store(frame, Ordering::Relaxed);
    }
}
