use nih_plug::prelude::{Editor, Enum, EnumParam, Param};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{ViziaState, ViziaTheming, create_vizia_editor};

use crate::{SineParams, Waveform};

pub const NOTO_SANS: &str = "Noto Sans";

#[derive(Lens)]
struct Data {
    params: Arc<SineParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (650, 550))
}

fn waveform_dropdown<L>(
    cx: &mut Context,
    params: L,
    map: impl Fn(&SineParams) -> &EnumParam<Waveform> + Copy + Send + Sync + 'static,
) -> Handle<'_, impl View>
where
    L: Lens<Target = Arc<SineParams>> + Clone + 'static + Send + Sync,
{
    Dropdown::new(
        cx,
        {
            let params = params.clone();
            move |cx| {
                Button::new(cx, |cx| {
                    HStack::new(cx, move |cx| {
                        Label::new(
                            cx,
                            params.clone().map(move |p| {
                                let param = map(&*p);
                                waveform_to_str(&param.value()).to_string()
                            }),
                        )
                        .width(Pixels(70.0))
                        .height(Pixels(25.0))
                        .text_align(TextAlign::Center)
                        .font_size(12.0)
                        .color(Color::white())
                        .alignment(Alignment::Center);

                        Label::new(cx, "▼")
                            .width(Pixels(15.0))
                            .height(Pixels(25.0))
                            .alignment(Alignment::Center)
                            .text_align(TextAlign::Center)
                            .color(Color::white())
                            .font_size(10.0);
                    })
                    .space(Pixels(5.0))
                })
                .width(Pixels(100.0))
                .height(Pixels(25.0))
                .background_color(Color::rgb(60, 80, 120))
                .corner_radius(Pixels(3.0))
                .cursor(CursorIcon::Hand)
                .on_press(move |cx| {
                    cx.emit(PopupEvent::Switch);
                });
            }
        },
        move |cx| {
            Binding::new(cx, params.clone(), move |cx, params_lens| {
                let params_data = params_lens.get(cx);
                let current_param = map(&*params_data);
                let current_value = current_param.value();

                VStack::new(cx, |cx| {
                    for option in [
                        Waveform::Sine,
                        Waveform::Square,
                        Waveform::Triangle,
                        Waveform::Sawtooth,
                    ] {
                        Button::new(cx, |cx| {
                            HStack::new(cx, |cx| {
                                Label::new(cx, if option == current_value { "✓" } else { " " })
                                    .width(Pixels(15.0))
                                    .font_size(10.0)
                                    .color(Color::white());

                                Label::new(cx, waveform_to_str(&option))
                                    .width(Pixels(90.0))
                                    .text_align(TextAlign::Left)
                                    .font_size(11.0)
                                    .color(Color::white());
                            })
                            .space(Pixels(5.0))
                        })
                        .width(Pixels(120.0))
                        .height(Pixels(22.0))
                        .background_color(if option == current_value {
                            Color::rgb(90, 110, 150)
                        } else {
                            Color::rgb(70, 90, 130)
                        })
                        .corner_radius(Pixels(2.0))
                        .cursor(CursorIcon::Hand)
                        .on_press({
                            let params_for_press = params.clone(); // ← Clone the lens
                            move |cx| {
                                // Use lens.get(cx) instead of cx.data::<Data>()
                                let params_arc = params_for_press.get(cx); // ← Direct lens access
                                let param = map(&*params_arc);
                                let param_ptr = param.as_ptr();
                                let normalized_value = param.preview_normalized(option);

                                cx.emit(RawParamEvent::BeginSetParameter(param_ptr));
                                cx.emit(RawParamEvent::SetParameterNormalized(
                                    param_ptr,
                                    normalized_value,
                                ));
                                cx.emit(RawParamEvent::EndSetParameter(param_ptr));

                                cx.emit(PopupEvent::Close);
                            }
                        });
                    }
                })
                .background_color(Color::rgb(50, 60, 90))
                .corner_radius(Pixels(3.0))
                .border_width(Pixels(1.0))
                .border_color(Color::rgb(80, 100, 140));
            });
        },
    )
    .placement(Placement::Bottom)
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
        vizia_plug::widgets::register_theme(cx);

        Data {
            params: params.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Triple Oscillator Synth")
                .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
                .font_weight(FontWeightKeyword::Bold)
                .font_size(22.0)
                .height(Pixels(40.0))
                .color(Color::rgb(240, 240, 240));

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

                        waveform_dropdown(cx, Data::params, |p| &p.waveform1)
                            .width(Pixels(100.0))
                            .height(Pixels(25.0));
                    })
                    .height(Pixels(30.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.frequency1)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                    .height(Pixels(25.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.gain1)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                    .height(Pixels(25.0));
                });
            })
            .padding(Pixels(10.0))
            .background_color(Color::rgb(45, 45, 55))
            .corner_radius(Pixels(4.0));

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

                        waveform_dropdown(cx, Data::params, |p| &p.waveform2)
                            .width(Pixels(100.0))
                            .height(Pixels(25.0));
                    })
                    .height(Pixels(30.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.frequency2)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                    .height(Pixels(25.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.gain2)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                    .height(Pixels(25.0));
                });
            })
            .padding(Pixels(10.0))
            .background_color(Color::rgb(45, 55, 45))
            .corner_radius(Pixels(4.0));

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

                        waveform_dropdown(cx, Data::params, |p| &p.waveform3)
                            .width(Pixels(100.0))
                            .height(Pixels(25.0));
                    })
                    .height(Pixels(30.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Frequency")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.frequency3)
                            .width(Pixels(300.0))
                            .height(Pixels(20.0));
                    })
                    .height(Pixels(25.0));

                    HStack::new(cx, |cx| {
                        Label::new(cx, "Gain")
                            .width(Pixels(70.0))
                            .height(Pixels(20.0))
                            .font_size(12.0);
                        ParamSlider::new(cx, Data::params, |p| &p.gain3)
                            .width(Pixels(200.0))
                            .height(Pixels(20.0));
                    })
                    .height(Pixels(25.0));
                });
            })
            .padding(Pixels(10.0))
            .background_color(Color::rgb(55, 45, 45))
            .corner_radius(Pixels(4.0));
        })
        .space(Pixels(12.0))
        .padding(Pixels(15.0))
        .background_color(Color::rgb(30, 30, 35));
    })
}
