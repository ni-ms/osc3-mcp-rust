use nih_plug::prelude::Param;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;

const KNOB_CSS: &str = r#"
/* Space knobs so they never visually crowd */
.param-knob { margin: 3px; cursor: pointer; }
.param-knob:hover { filter: brightness(1.08); }
/* Optional: label alignment helpers if used under knobs */
.knob-col { align-items: center; justify-content: center; }
"#;

#[derive(Lens)]
pub struct ParamKnob {
    param_base: ParamWidgetBase,
    drag_active: bool,
    drag_start_y: f32,
    scrolled_lines: f32,
}

impl ParamKnob {
    pub fn new<L, Params, P, FMap>(
        cx: &mut Context,
        params: L,
        params_to_param: FMap,
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone,
        Params: 'static,
        P: Param + 'static,
        FMap: Fn(&Params) -> &P + Copy + 'static,
    {
        cx.add_stylesheet(KNOB_CSS).ok();

        Self {
            param_base: ParamWidgetBase::new(cx, params.clone(), params_to_param),
            drag_active: false,
            drag_start_y: 0.0,
            scrolled_lines: 0.0,
        }
        .build(cx, |cx| {
            ZStack::new(cx, |cx| {
                Element::new(cx)
                    .width(Percentage(100.0))
                    .height(Percentage(100.0))
                    .background_color(Color::rgb(45, 55, 72))
                    .border_width(Pixels(2.0))
                    .border_color(Color::rgb(99, 102, 241))
                    .corner_radius(Percentage(50.0));

                Element::new(cx)
                    .width(Percentage(83.33))
                    .height(Percentage(83.33))
                    .background_color(Color::rgb(66, 73, 91))
                    .corner_radius(Percentage(50.0))
                    .position_type(PositionType::Absolute)
                    .left(Percentage(8.33))
                    .top(Percentage(8.33));

                Element::new(cx)
                    .width(Percentage(10.0))
                    .height(Percentage(10.0))
                    .background_color(Color::rgb(156, 163, 175))
                    .corner_radius(Percentage(50.0))
                    .position_type(PositionType::Absolute)
                    .left(Percentage(45.0))
                    .top(Percentage(45.0));

                Binding::new(
                    cx,
                    ParamWidgetBase::make_lens(params, params_to_param, |param| {
                        param.modulated_normalized_value()
                    }),
                    move |cx, normalized_value| {
                        let angle = -135.0 + (normalized_value.get(cx) * 270.0);

                        ZStack::new(cx, |cx| {
                            ZStack::new(cx, |cx| {
                                let line_w = 3.0;
                                let line_h = 14.0;
                                Element::new(cx)
                                    .width(Pixels(line_w))
                                    .height(Pixels(line_h))
                                    .background_color(Color::rgb(239, 246, 255))
                                    .corner_radius(Pixels(1.5))
                                    .position_type(PositionType::Absolute)
                                    .left(Pixels(-line_w * 0.5))
                                    .top(Pixels(-line_h));
                            })
                            .position_type(PositionType::Absolute)
                            .left(Percentage(50.0))
                            .top(Percentage(50.0))
                            .width(Pixels(0.0))
                            .height(Pixels(0.0));
                        })
                        .position_type(PositionType::Absolute)
                        .left(Percentage(0.0))
                        .top(Percentage(0.0))
                        .width(Percentage(100.0))
                        .height(Percentage(100.0))
                        .rotate(Angle::Deg(angle + 90.0));
                    },
                );
            })
            .width(Percentage(100.0))
            .height(Percentage(100.0));
        })
        .class("param-knob")
    }
}

impl View for ParamKnob {
    fn element(&self) -> Option<&'static str> {
        Some("param-knob")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
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
                let sensitivity = 0.005;
                let current_value = self.param_base.unmodulated_normalized_value();
                let new_value = (current_value + drag_delta * sensitivity).clamp(0.0, 1.0);
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
