//! `ParamKnob` — a custom Skia-drawn rotary control bound to a `nih_plug`
//! parameter via [`ParamWidgetBase`].
//!
//! Unlike a stack of `Element`s, this paints itself in [`View::draw`]: a dim
//! background track arc, a bright value arc, a recessed knob body, an indicator
//! line, and a hover glow. The value arc and indicator are read from
//! `normalized_value`, which a `Binding` keeps in sync with the live parameter
//! (and requests a redraw) — that binding is what makes the knob "reactive".
//!
//! Colours come from CSS: `background-color` drives the track, `color` drives
//! the accent (so each oscillator can tint its knobs via an `accent-*` class).

use nih_plug::prelude::Param;
use vizia_plug::vizia::prelude::*;
use vizia_plug::vizia::vg;
use vizia_plug::widgets::param_base::ParamWidgetBase;

pub const KNOB_CSS: &str = r#"
    .param-knob {
        width: 44px;
        height: 44px;
        background-color: #2A2A33;  /* track arc */
        color: #818CF8;             /* default accent (indigo) */
        cursor: hand;
    }

    /* Per-section accent tints, applied by the editor. */
    .param-knob.accent-indigo  { color: #818CF8; }
    .param-knob.accent-cyan    { color: #38BDF8; }
    .param-knob.accent-emerald { color: #22C55E; }
    .param-knob.accent-rose    { color: #F43F5E; }
    .param-knob.accent-purple  { color: #A855F7; }
"#;

/// Arc geometry, in Skia degrees (0° = 3 o'clock, positive = clockwise). A 270°
/// sweep starting at 135° leaves a symmetric gap at the bottom (6 o'clock).
const ARC_START: f32 = 135.0;
const ARC_SWEEP: f32 = 270.0;

#[derive(Lens)]
pub struct ParamKnob {
    param_base: ParamWidgetBase,
    hovered: bool,
    drag_active: bool,
    drag_start_y: f32,
    scrolled_lines: f32,
}

impl ParamKnob {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        let param_base = ParamWidgetBase::new(cx, params.clone(), params_to_param);

        let mut handle = Self {
            param_base,
            hovered: false,
            drag_active: false,
            drag_start_y: 0.0,
            scrolled_lines: 0.0,
        }
        .build(cx, |_| {})
        .class("param-knob");

        // Observe the live parameter and request a redraw whenever it changes
        // (host automation, AI writes, or our own gestures all flow through
        // here). `draw` reads the current value straight from `param_base`, so
        // this binding only has to mark the view dirty.
        let entity = handle.entity();
        let value_lens = ParamWidgetBase::make_lens(params, params_to_param, |p| {
            p.modulated_normalized_value()
        });
        Binding::new(handle.context(), value_lens, move |cx, _value| {
            cx.needs_redraw(entity);
        });

        handle
    }
}

impl View for ParamKnob {
    fn element(&self) -> Option<&'static str> {
        Some("param-knob")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let bounds = cx.bounds();
        let size = bounds.w.min(bounds.h);
        if size <= 0.0 {
            return;
        }

        let opacity = cx.opacity();
        let track_color = cx.background_color();
        let accent_color = cx.font_color();
        let accent = vg::Color::from_argb(255, accent_color.r(), accent_color.g(), accent_color.b());
        let track = vg::Color::from_argb(255, track_color.r(), track_color.g(), track_color.b());

        let cx0 = bounds.x + bounds.w * 0.5;
        let cy0 = bounds.y + bounds.h * 0.5;

        let stroke = (size * 0.11).max(2.5);
        let radius = size * 0.5 - stroke * 0.5 - 1.0;
        let oval = vg::Rect::new(cx0 - radius, cy0 - radius, cx0 + radius, cy0 + radius);

        let value_sweep = ARC_SWEEP * self.param_base.modulated_normalized_value().clamp(0.0, 1.0);

        // Hover glow — a soft accent ring drawn behind everything.
        if self.hovered {
            let mut glow = vg::Paint::default();
            glow.set_anti_alias(true);
            glow.set_style(vg::PaintStyle::Stroke);
            glow.set_stroke_width(stroke * 1.7);
            glow.set_stroke_cap(vg::PaintCap::Round);
            glow.set_color(vg::Color::from_argb(70, accent.r(), accent.g(), accent.b()));
            glow.set_alpha_f(glow.alpha_f() * opacity);
            canvas.draw_arc(oval, ARC_START, ARC_SWEEP, false, &glow);
        }

        // Background track arc (full sweep, dim).
        let mut track_paint = vg::Paint::default();
        track_paint.set_anti_alias(true);
        track_paint.set_style(vg::PaintStyle::Stroke);
        track_paint.set_stroke_width(stroke);
        track_paint.set_stroke_cap(vg::PaintCap::Round);
        track_paint.set_color(track);
        track_paint.set_alpha_f(opacity);
        canvas.draw_arc(oval, ARC_START, ARC_SWEEP, false, &track_paint);

        // Active value arc (accent).
        if value_sweep > 0.0 {
            let mut value_paint = vg::Paint::default();
            value_paint.set_anti_alias(true);
            value_paint.set_style(vg::PaintStyle::Stroke);
            value_paint.set_stroke_width(stroke);
            value_paint.set_stroke_cap(vg::PaintCap::Round);
            value_paint.set_color(accent);
            value_paint.set_alpha_f(opacity);
            canvas.draw_arc(oval, ARC_START, value_sweep, false, &value_paint);
        }

        // Recessed knob body.
        let body_radius = radius - stroke * 0.95;
        if body_radius > 0.0 {
            let mut body = vg::Paint::default();
            body.set_anti_alias(true);
            body.set_style(vg::PaintStyle::Fill);
            body.set_color(vg::Color::from_argb(255, 26, 26, 32));
            body.set_alpha_f(opacity);
            canvas.draw_circle((cx0, cy0), body_radius, &body);

            // Thin rim to lift the body off the background.
            let mut rim = vg::Paint::default();
            rim.set_anti_alias(true);
            rim.set_style(vg::PaintStyle::Stroke);
            rim.set_stroke_width((size * 0.02).max(1.0));
            rim.set_color(vg::Color::from_argb(255, 48, 48, 58));
            rim.set_alpha_f(opacity);
            canvas.draw_circle((cx0, cy0), body_radius, &rim);

            // Indicator line pointing to the current value.
            let angle = (ARC_START + value_sweep).to_radians();
            let (sin, cos) = angle.sin_cos();
            let r_inner = body_radius * 0.30;
            let r_outer = body_radius * 0.88;
            let mut indicator_path = vg::Path::new();
            indicator_path.move_to((cx0 + cos * r_inner, cy0 + sin * r_inner));
            indicator_path.line_to((cx0 + cos * r_outer, cy0 + sin * r_outer));

            let mut indicator = vg::Paint::default();
            indicator.set_anti_alias(true);
            indicator.set_style(vg::PaintStyle::Stroke);
            indicator.set_stroke_width((size * 0.06).max(1.5));
            indicator.set_stroke_cap(vg::PaintCap::Round);
            indicator.set_color(accent);
            indicator.set_alpha_f(opacity);
            canvas.draw_path(&indicator_path, &indicator);
        }
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseEnter => {
                self.hovered = true;
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave => {
                self.hovered = false;
                cx.needs_redraw();
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                cx.capture();
                cx.set_active(true);
                self.drag_start_y = cx.mouse().cursor_y;
                self.drag_active = true;
                self.param_base.begin_set_parameter(cx);
                meta.consume();
            }
            WindowEvent::MouseMove(_, y) if self.drag_active => {
                let drag_delta = self.drag_start_y - y;
                // Finer control while holding Shift.
                let sensitivity = if cx.modifiers().shift() { 0.0008 } else { 0.005 };
                let current_value = self.param_base.unmodulated_normalized_value();
                let new_value = (current_value + drag_delta * sensitivity).clamp(0.0, 1.0);
                self.drag_start_y = *y;
                self.param_base.set_normalized_value(cx, new_value);
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.drag_active => {
                cx.release();
                cx.set_active(false);
                self.drag_active = false;
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                self.param_base.begin_set_parameter(cx);
                self.param_base
                    .set_normalized_value(cx, self.param_base.default_normalized_value());
                self.param_base.end_set_parameter(cx);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, scroll_y) => {
                self.scrolled_lines += scroll_y;
                if self.scrolled_lines.abs() >= 1.0 {
                    self.param_base.begin_set_parameter(cx);
                    let current_value = self.param_base.unmodulated_normalized_value();
                    let scroll_sensitivity = 0.02;
                    let new_value = if self.scrolled_lines >= 1.0 {
                        self.scrolled_lines -= 1.0;
                        (current_value + scroll_sensitivity).clamp(0.0, 1.0)
                    } else {
                        self.scrolled_lines += 1.0;
                        (current_value - scroll_sensitivity).clamp(0.0, 1.0)
                    };
                    self.param_base.set_normalized_value(cx, new_value);
                    self.param_base.end_set_parameter(cx);
                }
                meta.consume();
            }
            _ => {}
        });
    }
}
