//! Output-level metering: a real-time-safe atomic the audio thread writes to,
//! and a Skia-drawn [`Meter`] view that reads it on a redraw timer.
//!
//! The audio thread publishes a *decaying block peak* (linear gain) into
//! [`PeakMeter`] via a single relaxed atomic store per process block — no locks,
//! no allocation, so it is safe to call from `SineSynth::process`. The GUI never
//! mutates state in `draw`; it just samples the atomic every frame, so the two
//! threads never contend.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;

/// Lock-free shared output level. Stores an `f32` linear-gain peak in the bit
/// pattern of an `AtomicU32` so the audio thread can publish it allocation-free.
#[derive(Debug)]
pub struct PeakMeter {
    /// Linear-gain peak, bit-cast into a u32.
    bits: AtomicU32,
}

impl Default for PeakMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl PeakMeter {
    pub fn new() -> Self {
        Self {
            bits: AtomicU32::new(0),
        }
    }

    /// Publish the latest peak. Real-time-safe: one relaxed store, no alloc.
    #[inline]
    pub fn store(&self, peak: f32) {
        self.bits.store(peak.to_bits(), Ordering::Relaxed);
    }

    /// Read the published peak (linear gain).
    #[inline]
    pub fn load(&self) -> f32 {
        f32::from_bits(self.bits.load(Ordering::Relaxed))
    }
}

/// CSS for the meter. Colours are read from `draw` directly (zone-based), so the
/// stylesheet only governs sizing/rounding here.
pub const METER_CSS: &str = r#"
    .level-meter {
        width: 120px;
        height: 8px;
        corner-radius: 4px;
        background-color: #0E0E12;
        border-width: 1px;
        border-color: #2D2D34;
    }
"#;

/// Floor of the meter's dB scale. Levels at or below this read as empty.
const DB_FLOOR: f32 = -60.0;
/// Redraw cadence for the animated fill (~30 fps).
const REFRESH: Duration = Duration::from_millis(33);

/// An animated horizontal output meter. Reads [`PeakMeter`] each redraw tick and
/// paints a green→amber→red fill that tracks the published (audio-decayed) peak.
pub struct Meter {
    peak: Arc<PeakMeter>,
}

impl Meter {
    pub fn new(cx: &mut Context, peak: Arc<PeakMeter>) -> Handle<'_, Self> {
        Self { peak }
            .build(cx, |cx| {
                // A free-running timer that simply marks the view dirty; the
                // fresh atomic value is sampled in `draw`. Timer events target
                // the current (meter) view, so this is self-contained.
                let timer = cx.add_timer(REFRESH, None, |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.needs_redraw();
                    }
                });
                cx.start_timer(timer);
            })
            .class("level-meter")
    }
}

impl View for Meter {
    fn element(&self) -> Option<&'static str> {
        Some("level-meter")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }

        // Linear peak -> dB -> normalized [0, 1] across the meter's dB window.
        let peak = self.peak.load().max(0.0);
        let db = 20.0 * peak.max(1e-6).log10();
        let norm = ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0);

        let radius = bounds.h * 0.5;

        // Inset the fill slightly so it sits inside the CSS border.
        let pad = 1.0;
        let track_w = bounds.w - pad * 2.0;
        let fill_w = (track_w * norm).max(0.0);

        if fill_w <= 0.0 {
            return;
        }

        // Zone colour: green up to -12 dB, amber to -3 dB, red above.
        let color = if db >= -3.0 {
            vg::Color::from_argb(255, 244, 63, 94) // rose/red
        } else if db >= -12.0 {
            vg::Color::from_argb(255, 251, 191, 36) // amber
        } else {
            vg::Color::from_argb(255, 34, 197, 94) // emerald
        };

        let rect = vg::Rect::new(
            bounds.x + pad,
            bounds.y + pad,
            bounds.x + pad + fill_w,
            bounds.y + bounds.h - pad,
        );

        let mut paint = vg::Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(vg::PaintStyle::Fill);
        paint.set_color(color);
        paint.set_alpha_f(cx.opacity());
        canvas.draw_round_rect(rect, radius, radius, &paint);
    }
}
