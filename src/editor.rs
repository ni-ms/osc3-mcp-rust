use nih_plug::prelude::{Editor, Enum, EnumParam, Param};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{ViziaState, ViziaTheming, create_vizia_editor};

use crate::{SineParams, Waveform};
pub const NOTO_SANS: &str = "Noto Sans";

struct ColorPalette;
impl ColorPalette {
    pub const BACKGROUND: Color = Color::rgb(18, 18, 22);
    pub const SURFACE: Color = Color::rgb(24, 24, 28);
    pub const SURFACE_ELEVATED: Color = Color::rgb(32, 32, 38);

    pub const PRIMARY: Color = Color::rgb(99, 102, 241);
    pub const PRIMARY_HOVER: Color = Color::rgb(79, 82, 221);
    pub const PRIMARY_LIGHT: Color = Color::rgb(165, 180, 252);

    pub const OSC1_ACCENT: Color = Color::rgb(59, 130, 246);
    pub const OSC2_ACCENT: Color = Color::rgb(34, 197, 94);
    pub const OSC3_ACCENT: Color = Color::rgb(239, 68, 68);

    pub const TEXT_PRIMARY: Color = Color::rgb(248, 250, 252);
    pub const TEXT_SECONDARY: Color = Color::rgb(148, 163, 184);
    pub const TEXT_MUTED: Color = Color::rgb(100, 116, 139);

    pub const BORDER: Color = Color::rgb(51, 65, 85);
    pub const HOVER: Color = Color::rgb(30, 41, 59);
    pub const ACTIVE: Color = Color::rgb(15, 23, 42);
}

#[derive(Lens)]
struct Data {
    params: Arc<SineParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (600, 650)) // Increased height to accommodate new sliders
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
                            .text_align(TextAlign::Center)
                            .font_size(11.0)
                            .color(ColorPalette::TEXT_PRIMARY);

                        Label::new(cx, "▼")
                            .font_size(9.0)
                            .color(ColorPalette::TEXT_SECONDARY);
                    })
                        .space(Pixels(4.0))
                })
                    .width(Pixels(90.0))
                    .height(Pixels(24.0))
                    .background_color(ColorPalette::SURFACE_ELEVATED)
                    .border_width(Pixels(1.0))
                    .border_color(ColorPalette::BORDER)
                    .corner_radius(Pixels(4.0))
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
                            Label::new(cx, waveform_to_str(&option))
                                .font_size(11.0)
                                .color(ColorPalette::TEXT_PRIMARY)
                        })
                            .width(Pixels(100.0))
                            .height(Pixels(22.0))
                            .background_color(if option == current_value {
                                ColorPalette::PRIMARY
                            } else {
                                Color::transparent()
                            })
                            .cursor(CursorIcon::Hand)
                            .on_press({
                                let params_for_press = params.clone();
                                move |cx| {
                                    let params_arc = params_for_press.get(cx);
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
                    .padding(Pixels(4.0))
                    .background_color(ColorPalette::SURFACE)
                    .corner_radius(Pixels(6.0))
                    .border_width(Pixels(1.0))
                    .border_color(ColorPalette::BORDER);
            });
        },
    )
        .placement(Placement::Bottom)
}

fn create_oscillator_section(
    cx: &mut Context,
    title: &str,
    accent_color: Color,
    waveform_map: impl Fn(&SineParams) -> &EnumParam<Waveform> + Copy + Send + Sync + 'static,
    freq_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
    gain_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
    phase_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
    detune_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Element::new(cx)
                .width(Pixels(2.0))
                .height(Pixels(14.0))
                .background_color(accent_color);

            Label::new(cx, title)
                .font_size(12.0)
                .font_weight(FontWeightKeyword::Medium)
                .color(ColorPalette::TEXT_PRIMARY);
        })
            .space(Pixels(6.0))
            .height(Pixels(18.0));

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, "Wave")
                    .width(Pixels(50.0))
                    .font_size(11.0)
                    .color(ColorPalette::TEXT_PRIMARY);

                waveform_dropdown(cx, Data::params, waveform_map);
            })
                .height(Pixels(26.0))
                .alignment(Alignment::Center);

            VStack::new(cx, |cx| {
                Label::new(cx, "Frequency")
                    .font_size(11.0)
                    .color(ColorPalette::TEXT_PRIMARY)
                    .height(Pixels(16.0));

                ParamSlider::new(cx, Data::params, freq_map)
                    .height(Pixels(8.0))
                    .width(Stretch(1.0));
            });

            VStack::new(cx, |cx| {
                Label::new(cx, "Detune")
                    .font_size(11.0)
                    .color(ColorPalette::TEXT_PRIMARY)
                    .height(Pixels(16.0));

                ParamSlider::new(cx, Data::params, detune_map)
                    .height(Pixels(8.0))
                    .width(Stretch(1.0));
            });

            VStack::new(cx, |cx| {
                Label::new(cx, "Phase Offset")
                    .font_size(11.0)
                    .color(ColorPalette::TEXT_PRIMARY)
                    .height(Pixels(16.0));

                ParamSlider::new(cx, Data::params, phase_map)
                    .height(Pixels(8.0))
                    .width(Stretch(1.0));
            });

            VStack::new(cx, |cx| {
                Label::new(cx, "Gain")
                    .font_size(11.0)
                    .color(ColorPalette::TEXT_PRIMARY)
                    .height(Pixels(16.0));

                ParamSlider::new(cx, Data::params, gain_map)
                    .height(Pixels(8.0))
                    .width(Stretch(1.0));
            });
        })
            .space(Pixels(6.0));
    })
        .padding(Pixels(10.0))
        .background_color(ColorPalette::SURFACE)
        .border_width(Pixels(1.0))
        .border_color(ColorPalette::BORDER)
        .corner_radius(Pixels(8.0));
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
            Label::new(cx, "TripleOsc")
                .font_family(vec![FamilyOwned::Named(String::from(NOTO_SANS))])
                .font_weight(FontWeightKeyword::Bold)
                .font_size(16.0)
                .color(ColorPalette::TEXT_PRIMARY)
                .text_align(TextAlign::Center)
                .height(Pixels(24.0));

            VStack::new(cx, |cx| {
                create_oscillator_section(
                    cx,
                    "Oscillator 1",
                    ColorPalette::OSC1_ACCENT,
                    |p| &p.waveform1,
                    |p| &p.frequency1,
                    |p| &p.gain1,
                    |p| &p.phase1,      // Add phase parameter mapping
                    |p| &p.detune1,     // Add detune parameter mapping
                );

                create_oscillator_section(
                    cx,
                    "Oscillator 2",
                    ColorPalette::OSC2_ACCENT,
                    |p| &p.waveform2,
                    |p| &p.frequency2,
                    |p| &p.gain2,
                    |p| &p.phase2,      // Add phase parameter mapping
                    |p| &p.detune2,     // Add detune parameter mapping
                );

                create_oscillator_section(
                    cx,
                    "Oscillator 3",
                    ColorPalette::OSC3_ACCENT,
                    |p| &p.waveform3,
                    |p| &p.frequency3,
                    |p| &p.gain3,
                    |p| &p.phase3,      // Add phase parameter mapping
                    |p| &p.detune3,     // Add detune parameter mapping
                );
            })
                .space(Pixels(8.0));
        })
            .padding(Pixels(12.0))
            .background_color(ColorPalette::BACKGROUND)
            .space(Pixels(8.0));
    })
}
