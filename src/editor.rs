use nih_plug::prelude::{Editor, EnumParam, Param};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::knob::ParamKnob;
use crate::{SineParams, Waveform};

use crate::tab_switcher::{TabDefinition, TabSwitcher};

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
    ViziaState::new(|| (600, 650))
}

fn octave_counter<L>(
    cx: &mut Context,
    params: L,
    octave_map: impl Fn(&SineParams) -> &nih_plug::prelude::IntParam + Copy + Send + Sync + 'static,
) -> Handle<'_, impl View>
where
    L: Lens<Target = Arc<SineParams>> + Clone + 'static + Send + Sync,
{
    VStack::new(cx, |cx| {
        Label::new(cx, "Octave")
            .font_size(10.0)
            .color(ColorPalette::TEXT_PRIMARY)
            .height(Pixels(12.0))
            .text_align(TextAlign::Center);

        HStack::new(cx, |cx| {
            Button::new(cx, |cx| {
                Label::new(cx, "−")
                    .font_size(14.0)
                    .color(ColorPalette::TEXT_PRIMARY)
            })
                .width(Pixels(18.0))
                .height(Pixels(22.0))
                .background_color(ColorPalette::SURFACE_ELEVATED)
                .border_width(Pixels(1.0))
                .border_color(ColorPalette::BORDER)
                .corner_radius(Pixels(3.0))
                .cursor(CursorIcon::Hand)
                .on_press({
                    let params = params.clone();
                    move |cx| {
                        let params_arc = params.get(cx);
                        let param = octave_map(&*params_arc);
                        let param_ptr = param.as_ptr();
                        let current_value = param.modulated_plain_value();
                        let new_value = (current_value - 1).max(-4);
                        let normalized_value = param.preview_normalized(new_value);

                        cx.emit(RawParamEvent::BeginSetParameter(param_ptr));
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            param_ptr,
                            normalized_value,
                        ));
                        cx.emit(RawParamEvent::EndSetParameter(param_ptr));
                    }
                });

            Label::new(
                cx,
                params.clone().map(move |p| {
                    let param = octave_map(&*p);
                    let value = param.modulated_plain_value();
                    if value >= 0 {
                        format!("+{}", value)
                    } else {
                        format!("{}", value)
                    }
                }),
            )
                .width(Pixels(32.0))
                .height(Pixels(22.0))
                .background_color(ColorPalette::SURFACE)
                .border_width(Pixels(1.0))
                .border_color(ColorPalette::BORDER)
                .font_size(10.0)
                .color(ColorPalette::TEXT_PRIMARY)
                .text_align(TextAlign::Center);

            Button::new(cx, |cx| {
                Label::new(cx, "+")
                    .font_size(14.0)
                    .color(ColorPalette::TEXT_PRIMARY)
            })
                .width(Pixels(18.0))
                .height(Pixels(22.0))
                .background_color(ColorPalette::SURFACE_ELEVATED)
                .border_width(Pixels(1.0))
                .border_color(ColorPalette::BORDER)
                .corner_radius(Pixels(3.0))
                .cursor(CursorIcon::Hand)
                .on_press({
                    let params = params.clone();
                    move |cx| {
                        let params_arc = params.get(cx);
                        let param = octave_map(&*params_arc);
                        let param_ptr = param.as_ptr();
                        let current_value = param.modulated_plain_value();
                        let new_value = (current_value + 1).min(4);
                        let normalized_value = param.preview_normalized(new_value);

                        cx.emit(RawParamEvent::BeginSetParameter(param_ptr));
                        cx.emit(RawParamEvent::SetParameterNormalized(
                            param_ptr,
                            normalized_value,
                        ));
                        cx.emit(RawParamEvent::EndSetParameter(param_ptr));
                    }
                });
        })
            .space(Pixels(0.0))
            .alignment(Alignment::Center);
    })
        .space(Pixels(2.0))
        .alignment(Alignment::Center)
        .width(Pixels(68.0))
        .height(Pixels(36.0))
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
    phase_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam
    + Copy
    + Send
    + Sync
    + 'static,
    detune_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam
    + Copy
    + Send
    + Sync
    + 'static,
    octave_map: impl Fn(&SineParams) -> &nih_plug::prelude::IntParam + Copy + Send + Sync + 'static,

    unison_voices_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::IntParam + Copy + Send + Sync + 'static,
    unison_detune_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
    unison_blend_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
    unison_volume_map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
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


        let osc_tabs = vec![
            TabDefinition::new("wave", "Wave").with_width(80.0),
            TabDefinition::new("unison", "Unison").with_width(80.0),
        ];

        TabSwitcher::new(cx, osc_tabs, move |cx, tab_id, _index| {
            match tab_id {
                "wave" => {

                    VStack::new(cx, |cx| {

                        HStack::new(cx, |cx| {
                            Label::new(cx, "Wave")
                                .width(Pixels(50.0))
                                .font_size(11.0)
                                .color(ColorPalette::TEXT_PRIMARY);
                            waveform_dropdown(cx, Data::params, waveform_map);

                            Element::new(cx).width(Pixels(15.0));

                            octave_counter(cx, Data::params, octave_map);
                        })
                            .height(Pixels(40.0))
                            .alignment(Alignment::Center);


                        HStack::new(cx, |cx| {
                            VStack::new(cx, |cx| {
                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Frequency")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, freq_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));

                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Detune")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, detune_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));
                            })
                                .space(Pixels(10.0));

                            VStack::new(cx, |cx| {
                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Phase")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, phase_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));

                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Gain")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, gain_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));
                            })
                                .space(Pixels(10.0));
                        })
                            .space(Pixels(15.0))
                            .alignment(Alignment::Center);
                    })
                        .space(Pixels(8.0));
                }

                "unison" => {

                    VStack::new(cx, |cx| {
                        Label::new(cx, "Unison Settings")
                            .font_size(11.0)
                            .color(ColorPalette::TEXT_SECONDARY)
                            .height(Pixels(16.0))
                            .text_align(TextAlign::Center);


                        HStack::new(cx, |cx| {
                            VStack::new(cx, |cx| {
                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Voices")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, unison_voices_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));

                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Detune")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, unison_detune_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));
                            })
                                .space(Pixels(10.0));

                            VStack::new(cx, |cx| {
                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Blend")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, unison_blend_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));

                                VStack::new(cx, |cx| {
                                    Label::new(cx, "Volume")
                                        .font_size(10.0)
                                        .color(ColorPalette::TEXT_PRIMARY)
                                        .height(Pixels(12.0))
                                        .text_align(TextAlign::Center);
                                    ParamKnob::new(cx, Data::params, unison_volume_map)
                                        .width(Pixels(40.0))
                                        .height(Pixels(40.0));
                                })
                                    .space(Pixels(2.0))
                                    .alignment(Alignment::Center)
                                    .width(Pixels(50.0))
                                    .height(Pixels(60.0));
                            })
                                .space(Pixels(10.0));
                        })
                            .space(Pixels(15.0))
                            .alignment(Alignment::Center);
                    })
                        .space(Pixels(8.0));
                }

                _ => {
                    Label::new(cx, "Unknown Tab")
                        .font_size(12.0)
                        .color(ColorPalette::TEXT_PRIMARY)
                        .text_align(TextAlign::Center);
                }
            }
        })
            .height(Pixels(160.0));
    })
        .padding(Pixels(8.0))
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
        register_theme(cx);

        cx.add_stylesheet("assets/knob.css")
            .expect("Failed to load stylesheet");
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

            let tabs = vec![
                TabDefinition::new("oscillators", "Oscillators").with_width(140.0),
                TabDefinition::new("envelope", "Envelope").with_width(120.0),
            ];

            TabSwitcher::new(cx, tabs, |cx, tab_id, _index| match tab_id {
                "oscillators" => {
                    VStack::new(cx, |cx| {
                        create_oscillator_section(
                            cx,
                            "Oscillator 1",
                            ColorPalette::OSC1_ACCENT,
                            |p| &p.waveform1,
                            |p| &p.frequency1,
                            |p| &p.gain1,
                            |p| &p.phase1,
                            |p| &p.detune1,
                            |p| &p.octave1,

                            |p| &p.unison_voices1,
                            |p| &p.unison_detune1,
                            |p| &p.unison_blend1,
                            |p| &p.unison_volume1,    
                        );

                        create_oscillator_section(
                            cx,
                            "Oscillator 2",
                            ColorPalette::OSC2_ACCENT,
                            |p| &p.waveform2,
                            |p| &p.frequency2,
                            |p| &p.gain2,
                            |p| &p.phase2,
                            |p| &p.detune2,
                            |p| &p.octave2,
                            |p| &p.unison_voices2,
                            |p| &p.unison_detune2,
                            |p| &p.unison_blend2,
                            |p| &p.unison_volume2,
                        );

                        create_oscillator_section(
                            cx,
                            "Oscillator 3",
                            ColorPalette::OSC3_ACCENT,
                            |p| &p.waveform3,
                            |p| &p.frequency3,
                            |p| &p.gain3,
                            |p| &p.phase3,
                            |p| &p.detune3,
                            |p| &p.octave3,
                            |p| &p.unison_voices3,
                            |p| &p.unison_detune3,
                            |p| &p.unison_blend3,
                            |p| &p.unison_volume3,
                        );
                    })
                        .space(Pixels(8.0));
                }

                "envelope" => {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Envelope Controls")
                            .font_size(14.0)
                            .font_weight(FontWeightKeyword::Medium)
                            .color(ColorPalette::TEXT_PRIMARY)
                            .text_align(TextAlign::Center);

                        VStack::new(cx, |cx| {
                            VStack::new(cx, |cx| {
                                Label::new(cx, "Attack")
                                    .font_size(11.0)
                                    .color(ColorPalette::TEXT_PRIMARY)
                                    .height(Pixels(16.0));

                                Element::new(cx)
                                    .height(Pixels(8.0))
                                    .width(Stretch(1.0))
                                    .background_color(ColorPalette::SURFACE_ELEVATED)
                                    .corner_radius(Pixels(4.0));
                            });

                            VStack::new(cx, |cx| {
                                Label::new(cx, "Decay")
                                    .font_size(11.0)
                                    .color(ColorPalette::TEXT_PRIMARY)
                                    .height(Pixels(16.0));

                                Element::new(cx)
                                    .height(Pixels(8.0))
                                    .width(Stretch(1.0))
                                    .background_color(ColorPalette::SURFACE_ELEVATED)
                                    .corner_radius(Pixels(4.0));
                            });

                            VStack::new(cx, |cx| {
                                Label::new(cx, "Sustain")
                                    .font_size(11.0)
                                    .color(ColorPalette::TEXT_PRIMARY)
                                    .height(Pixels(16.0));

                                Element::new(cx)
                                    .height(Pixels(8.0))
                                    .width(Stretch(1.0))
                                    .background_color(ColorPalette::SURFACE_ELEVATED)
                                    .corner_radius(Pixels(4.0));
                            });

                            VStack::new(cx, |cx| {
                                Label::new(cx, "Release")
                                    .font_size(11.0)
                                    .color(ColorPalette::TEXT_PRIMARY)
                                    .height(Pixels(16.0));

                                Element::new(cx)
                                    .height(Pixels(8.0))
                                    .width(Stretch(1.0))
                                    .background_color(ColorPalette::SURFACE_ELEVATED)
                                    .corner_radius(Pixels(4.0));
                            });
                        })
                            .space(Pixels(12.0))
                            .padding(Pixels(16.0))
                            .background_color(ColorPalette::SURFACE)
                            .border_width(Pixels(1.0))
                            .border_color(ColorPalette::BORDER)
                            .corner_radius(Pixels(8.0));
                    })
                        .space(Pixels(16.0));
                }

                _ => {
                    Label::new(cx, "Unknown Tab")
                        .font_size(14.0)
                        .color(ColorPalette::TEXT_PRIMARY)
                        .text_align(TextAlign::Center);
                }
            })
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        })
            .padding(Pixels(12.0))
            .background_color(ColorPalette::BACKGROUND)
            .space(Pixels(8.0));
    })
}
