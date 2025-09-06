use nih_plug::prelude::{Editor, Enum, EnumParam, Param};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::{SineParams, Waveform};

pub const NOTO_SANS: &str = "Noto Sans";

#[derive(Lens)]
struct Data {
    // Keep the Arc to the params in the app model so widgets can lens into it
    params: Arc<SineParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (650, 550))
}

use vizia_plug::widgets::ParamEvent;

#[derive(Lens, Debug, Clone, Copy, PartialEq, Eq)]
struct WaveformDropdown {
    is_open: bool,
}

impl WaveformDropdown {
    pub fn new<L, Params>(
        cx: &mut Context,
        params: L,
        params_to_param: impl Fn(&Params) -> &EnumParam<Waveform> + Copy + Send + 'static,
    ) -> Handle<Self>
    where
        L: Lens<Target = Params> + Clone + 'static,
        Params: 'static,
    {
        Self { is_open: false }
            .build(cx, |cx| {
                // Button area (label + caret)
                HStack::new(cx, |cx| {
                    // Current value label
                    Label::new(
                        cx,
                        params
                            .clone()
                            .map(move |p| {
                                let param = params_to_param(p);
                                waveform_to_str(&param.value()).to_string()
                            }),
                    )
                        .width(Pixels(80.0))
                        .height(Pixels(25.0))
                        .background_color(Color::rgb(60, 80, 120))
                        .corner_radius(Pixels(3.0))
                        .space(Stretch(1.0))
                        .font_size(12.0)
                        .color(Color::white());

                    // Caret
                    Label::new(cx, "▼")
                        .width(Pixels(15.0))
                        .height(Pixels(25.0))
                        .color(Color::white())
                        .font_size(10.0)
                        .space(Stretch(1.0));
                })
                    .width(Pixels(100.0))
                    .height(Pixels(25.0))
                    .background_color(Color::rgb(60, 80, 120))
                    .corner_radius(Pixels(3.0))
                    .cursor(CursorIcon::Hand)
                    .on_press(|cx| cx.emit(DropdownEvent::ToggleOpen));

                // Popup menu
                VStack::new(cx, |cx| {
                    for option in &[
                        Waveform::Sine,
                        Waveform::Square,
                        Waveform::Triangle,
                        Waveform::Sawtooth,
                    ] {
                        let option_copy = *option;
                        Label::new(cx, waveform_to_str(&option_copy))
                            .width(Pixels(100.0))
                            .height(Pixels(20.0))
                            .background_color(Color::rgb(70, 90, 130))
                            .corner_radius(Pixels(2.0))
                            .space(Stretch(1.0))
                            .font_size(11.0)
                            .color(Color::white())
                            .cursor(CursorIcon::Hand)
                            .on_press({
                                move |cx| {
                                    // Look up the params from the app model and update the EnumParam via ParamEvent
                                    if let Some(data) = cx.data::<Data>() {
                                        let param_ref = params_to_param(&*data.params);
                                        cx.emit(ParamEvent::BeginSetParameter(param_ref).upcast());
                                        cx.emit(ParamEvent::SetParameter(param_ref, option_copy).upcast());
                                        cx.emit(ParamEvent::EndSetParameter(param_ref).upcast());
                                    }
                                    cx.emit(DropdownEvent::ToggleOpen);
                                }
                            });
                    }
                })
                    .display(
                        WaveformDropdown::is_open.map(|open| if *open { Display::Flex } else { Display::None }),
                    )
                    // Anchor the popup under the button and keep it above siblings
                    .position_type(PositionType::Absolute)
                    .left(Pixels(0.0))
                    .top(Pixels(27.0))
                    .z_index(100)
                    .background_color(Color::rgb(50, 60, 90))
                    .corner_radius(Pixels(3.0))
                    .border_width(Pixels(1.0))
                    .border_color(Color::rgb(80, 100, 140));
            })
            // Keep the overall control in normal flow; only the popup is absolute
            .display(Display::Flex)
    }
}

#[derive(Debug)]
enum DropdownEvent {
    ToggleOpen,
}

impl View for WaveformDropdown {
    fn element(&self) -> Option<&'static str> {
        Some("waveform-dropdown")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        // Close on outside click
        event.map(|window_event, _| {
            if let WindowEvent::MouseDown(MouseButton::Left) = window_event {
                if self.is_open && !cx.hovered() {
                    self.is_open = false;
                }
            }
        });

        event.map(|dropdown_event, meta| match dropdown_event {
            DropdownEvent::ToggleOpen => {
                self.is_open = !self.is_open;
                meta.consume();
            }
        });
    }
}

fn waveform_to_str(w: &Waveform) -> &'static str {
    match w {
        Waveform::Sine => "Sine",
        Waveform::Square => "Square",
        Waveform::Triangle => "Triangle",
        Waveform::Sawtooth => "Sawtooth",
    }
}

pub(crate) fn create(
    params: Arc<SineParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // If using custom theming, register the default widget styles
        vizia_plug::widgets::register_theme(cx);

        Data { params: params.clone() }.build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Triple Oscillator Synth")
                .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
                .font_weight(FontWeightKeyword::Bold)
                .font_size(22.0)
                .height(Pixels(40.0))
                .color(Color::rgb(240, 240, 240))
                .space(Stretch(1.0));

            // Osc 1
            VStack::new(cx, |cx| {
                Label::new(cx, "Oscillator 1")
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(14.0)
                    .color(Color::rgb(150, 150, 255))
                    .height(Pixels(25.0));

                VStack::new(cx, |cx| {
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Waveform")
                            .width(Pixels(70.0))
                            .height(Pixels(25.0))
                            .font_size(12.0);

                        // Use the fixed dropdown widget for EnumParam<Waveform>
                        WaveformDropdown::new(cx, Data::params, |p| &p.waveform1)
                            .width(Pixels(100.0))
                            .height(Pixels(25.0));
                    })
                        .height(Pixels(30.0))
                        .space(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.frequency1)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                        .height(Pixels(25.0))
                        .space(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.gain1)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                        .height(Pixels(25.0))
                        .space(Stretch(1.0));
                })
                    .space(Stretch(1.0))
                    .space(Pixels(5.0));
            })
                .space(Stretch(1.0))
                .padding(Pixels(10.0))
                .background_color(Color::rgb(45, 45, 55))
                .corner_radius(Pixels(4.0));

            // Osc 2
            VStack::new(cx, |cx| {
                Label::new(cx, "Oscillator 2")
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(14.0)
                    .color(Color::rgb(150, 255, 150))
                    .height(Pixels(25.0));

                VStack::new(cx, |cx| {
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Waveform")
                            .width(Pixels(70.0))
                            .height(Pixels(25.0))
                            .font_size(12.0);

                        // Could use ParamButton for a two-state bool, but here it's an enum, so keep dropdown or a custom stepper if desired
                        WaveformDropdown::new(cx, Data::params, |p| &p.waveform2)
                            .width(Pixels(100.0))
                            .height(Pixels(25.0));
                    })
                        .height(Pixels(30.0))
                        .space(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.frequency2)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                        .height(Pixels(25.0))
                        .space(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.gain2)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                        .height(Pixels(25.0))
                        .space(Stretch(1.0));
                })
                    .space(Stretch(1.0))
                    .space(Pixels(5.0));
            })
                .space(Stretch(1.0))
                .padding(Pixels(10.0))
                .background_color(Color::rgb(45, 55, 45))
                .corner_radius(Pixels(4.0));

            // Osc 3
            VStack::new(cx, |cx| {
                Label::new(cx, "Oscillator 3")
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(14.0)
                    .color(Color::rgb(255, 150, 150))
                    .height(Pixels(25.0));

                VStack::new(cx, |cx| {
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Waveform")
                            .width(Pixels(70.0))
                            .height(Pixels(25.0))
                            .font_size(12.0);

                        WaveformDropdown::new(cx, Data::params, |p| &p.waveform3)
                            .width(Pixels(100.0))
                            .height(Pixels(25.0));
                    })
                        .height(Pixels(30.0))
                        .space(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.frequency3)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                        .height(Pixels(25.0))
                        .space(Stretch(1.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.gain3)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                        .height(Pixels(25.0))
                        .space(Stretch(1.0));
                })
                    .space(Stretch(1.0))
                    .space(Pixels(5.0));
            })
                .space(Stretch(1.0))
                .padding(Pixels(10.0))
                .background_color(Color::rgb(55, 45, 45))
                .corner_radius(Pixels(4.0));
        })
            .space(Pixels(12.0))
            .space(Stretch(1.0))
            .padding(Pixels(15.0))
            .background_color(Color::rgb(30, 30, 35));
    })
}
