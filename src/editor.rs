use crate::knob::ParamKnob;
use crate::{FilterMode, McpPluginState, OscillatorParams, SineParams, Waveform};
use nih_plug::prelude::{Editor, EnumParam, Param};
use std::sync::Arc;
use tokio::sync::RwLock;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

use crate::tab_switcher::{TabDefinition, TabSwitcher};

// --- MODERN COLOR PALETTE ---
struct ColorPalette;
impl ColorPalette {
    pub const BG_APP: Color = Color::rgb(10, 10, 12);
    pub const BG_CARD: Color = Color::rgb(22, 22, 26);
    pub const BG_CARD_ALT: Color = Color::rgb(28, 28, 34);

    pub const BORDER: Color = Color::rgb(45, 45, 52);
    pub const BORDER_LIGHT: Color = Color::rgb(60, 60, 70);

    pub const PRIMARY: Color = Color::rgb(99, 102, 241); // Indigo 500
    pub const PRIMARY_HOVER: Color = Color::rgb(129, 140, 248); // Indigo 400

    pub const OSC1_ACCENT: Color = Color::rgb(56, 189, 248); // Cyan
    pub const OSC2_ACCENT: Color = Color::rgb(34, 197, 94); // Emerald
    pub const OSC3_ACCENT: Color = Color::rgb(244, 63, 94); // Rose
    pub const FILTER_ACCENT: Color = Color::rgb(168, 85, 247); // Purple

    pub const TEXT_HIGH: Color = Color::rgb(248, 250, 252);
    pub const TEXT_MED: Color = Color::rgb(148, 163, 184);
    pub const TEXT_LOW: Color = Color::rgb(71, 85, 105);
}

#[derive(Lens)]
struct Data {
    params: Arc<SineParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (700, 720))
}

// --- MODERN STYLESHEET ---
const UI_STYLESHEET: &str = r#"
    .root {
        background-color: #0A0A0C;
    }

    .header {
        height: 60px;
        background-color: #16161A;
        border-bottom: 1px solid #2D2D34;
        padding-left: 20px;
        padding-right: 20px;
        align-items: center;
    }

    .app-title {
        color: #F8FAFC;
        font-weight: 800;
        font-size: 18px;
        letter-spacing: 1.2px;
    }

    .module-card {
        background-color: #16161A;
        border: 1px solid #2D2D34;
        border-radius: 8px;
        padding: 16px;
        margin-bottom: 12px;
    }

    .module-title {
        color: #F8FAFC;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 1px;
    }

    .octave-counter {
        background-color: #0F172A;
        border-radius: 4px;
        border: 1px solid #334155;
        overflow: hidden;
    }

    .counter-btn {
        width: 24px;
        height: 24px;
        background-color: transparent;
        color: #94A3B8;
        font-size: 14px;
    }

    .counter-btn:hover {
        background-color: #1E293B;
        color: #F8FAFC;
    }

    .counter-value {
        width: 35px;
        color: #818CF8;
        font-weight: 700;
        font-size: 11px;
        text-align: center;
    }

    .knob-stack {
        align-items: center;
        child-space: 4px;
    }

    .knob-label {
        font-size: 9px;
        font-weight: 600;
        color: #64748B;
        text-transform: uppercase;
        text-align: center;
    }

    .dropdown-trigger {
        background-color: #1C1C22;
        border: 1px solid #334155;
        border-radius: 4px;
    }
    
    .dropdown-trigger:hover {
        border-color: #6366F1;
    }

    .gap-16 { col-between: 16px; }
    .padding-20 { padding: 20px; }
"#;

// --- LOGIC HELPERS ---
fn adjust_octave(
    cx: &mut EventContext,
    params_arc: &Arc<SineParams>,
    map: impl Fn(&SineParams) -> &nih_plug::prelude::IntParam,
    delta: i32,
) {
    let param = map(&*params_arc);
    let ptr = param.as_ptr();
    let current = param.modulated_plain_value();
    let new = (current + delta).clamp(-4, 4);
    let norm = param.preview_normalized(new);

    cx.emit(RawParamEvent::BeginSetParameter(ptr));
    cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
    cx.emit(RawParamEvent::EndSetParameter(ptr));
}

fn waveform_to_str(w: &Waveform) -> &'static str {
    match w {
        Waveform::Sine => "Sine",
        Waveform::Square => "Square",
        Waveform::Triangle => "Triangle",
        Waveform::Sawtooth => "Sawtooth",
    }
}

fn filter_mode_to_str(mode: &FilterMode) -> &'static str {
    match mode {
        FilterMode::LowPass => "Low Pass",
        FilterMode::HighPass => "High Pass",
        FilterMode::BandPass => "Band Pass",
        FilterMode::Notch => "Notch",
    }
}

// --- CUSTOM WIDGETS ---

pub fn octave_counter<L>(
    cx: &mut Context,
    params: L,
    octave_map: impl Fn(&SineParams) -> &nih_plug::prelude::IntParam + Copy + Send + Sync + 'static,
) -> Handle<'_, impl View>
where
    L: Lens<Target = Arc<SineParams>> + Clone + 'static + Send + Sync,
{
    VStack::new(cx, |cx| {
        Label::new(cx, "OCTAVE").class("knob-label");
        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Label::new(cx, "−"))
                .class("counter-btn")
                .cursor(CursorIcon::Hand)
                .on_press({
                    let params = params.clone();
                    move |cx| {
                        let p = params.get(cx);
                        adjust_octave(cx, &p, octave_map, -1);
                    }
                });

            Label::new(
                cx,
                params.clone().map(move |p| {
                    let v = octave_map(&*p).modulated_plain_value();
                    if v >= 0 {
                        format!("+{}", v)
                    } else {
                        format!("{}", v)
                    }
                }),
            )
            .class("counter-value");

            Button::new(cx, |cx| Label::new(cx, "+"))
                .class("counter-btn")
                .cursor(CursorIcon::Hand)
                .on_press({
                    let params = params.clone();
                    move |cx| {
                        let p = params.get(cx);
                        adjust_octave(cx, &p, octave_map, 1);
                    }
                });
        })
        .class("octave-counter")
        .alignment(Alignment::Center);
    })
    .class("knob-stack")
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
                            params
                                .clone()
                                .map(move |p| waveform_to_str(&map(&*p).value()).to_string()),
                        )
                        .font_size(10.0)
                        .color(ColorPalette::TEXT_HIGH);
                        Label::new(cx, "▼")
                            .font_size(8.0)
                            .color(ColorPalette::TEXT_MED);
                    })
                    .padding_left(Pixels(10.0))
                    .padding_right(Pixels(10.0))
                })
                .class("dropdown-trigger")
                .width(Pixels(90.0))
                .height(Pixels(26.0))
                .on_press(move |cx| cx.emit(PopupEvent::Switch));
            }
        },
        move |cx| {
            VStack::new(cx, |cx| {
                for option in [
                    Waveform::Sine,
                    Waveform::Square,
                    Waveform::Triangle,
                    Waveform::Sawtooth,
                ] {
                    Button::new(cx, |cx| {
                        Label::new(cx, waveform_to_str(&option)).font_size(10.0)
                    })
                    .width(Stretch(1.0))
                    .height(Pixels(24.0))
                    .on_press({
                        let params = params.clone();
                        move |cx| {
                            let p_arc = params.get(cx);
                            let p = map(&*p_arc);
                            let ptr = p.as_ptr();
                            let norm = p.preview_normalized(option);
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                            cx.emit(PopupEvent::Close);
                        }
                    });
                }
            })
            .background_color(ColorPalette::BG_CARD_ALT)
            .border_width(Pixels(1.0))
            .border_color(ColorPalette::BORDER);
        },
    )
    .placement(Placement::Bottom)
}

fn filter_mode_dropdown<L>(
    cx: &mut Context,
    params: L,
    map: impl Fn(&SineParams) -> &EnumParam<FilterMode> + Copy + Send + Sync + 'static,
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
                            params
                                .clone()
                                .map(move |p| filter_mode_to_str(&map(&*p).value()).to_string()),
                        )
                        .font_size(10.0)
                        .color(ColorPalette::TEXT_HIGH);
                        Label::new(cx, "▼")
                            .font_size(8.0)
                            .color(ColorPalette::TEXT_MED);
                    })
                    .padding_left(Pixels(10.0))
                    .padding_right(Pixels(10.0))
                })
                .class("dropdown-trigger")
                .width(Pixels(110.0))
                .height(Pixels(26.0))
                .on_press(move |cx| cx.emit(PopupEvent::Switch));
            }
        },
        move |cx| {
            VStack::new(cx, |cx| {
                for option in [
                    FilterMode::LowPass,
                    FilterMode::HighPass,
                    FilterMode::BandPass,
                    FilterMode::Notch,
                ] {
                    Button::new(cx, |cx| {
                        Label::new(cx, filter_mode_to_str(&option)).font_size(10.0)
                    })
                    .width(Stretch(1.0))
                    .height(Pixels(24.0))
                    .on_press({
                        let params = params.clone();
                        let opt = option.clone();
                        move |cx| {
                            let p_arc = params.get(cx);
                            let p = map(&*p_arc);
                            let ptr = p.as_ptr();
                            let norm = p.preview_normalized(opt);
                            cx.emit(RawParamEvent::BeginSetParameter(ptr));
                            cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
                            cx.emit(RawParamEvent::EndSetParameter(ptr));
                            cx.emit(PopupEvent::Close);
                        }
                    });
                }
            })
            .background_color(ColorPalette::BG_CARD_ALT)
            .border_width(Pixels(1.0))
            .border_color(ColorPalette::BORDER);
        },
    )
    .placement(Placement::Bottom)
}

fn param_knob_block<L>(
    cx: &mut Context,
    label: &str,
    params: L,
    map: impl Fn(&Arc<SineParams>) -> &nih_plug::prelude::FloatParam + Copy + Send + Sync + 'static,
) where
    L: Lens<Target = Arc<SineParams>> + Clone + 'static + Send + Sync,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label).class("knob-label");
        ParamKnob::new(cx, params, map).size(Pixels(40.0));
    })
    .class("knob-stack");
}

/// Builds one oscillator module card. `osc` selects which of the three
/// oscillator param groups this section drives; every knob is derived from it,
/// so the three call sites differ only by selector and accent colour.
fn create_osc_section(
    cx: &mut Context,
    title: &str,
    accent: Color,
    osc: impl Fn(&SineParams) -> &OscillatorParams + Copy + Send + Sync + 'static,
) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Element::new(cx)
                .width(Pixels(3.0))
                .height(Pixels(14.0))
                .background_color(accent)
                .corner_radius(Pixels(1.5));
            Label::new(cx, title)
                .class("module-title")
                .padding_left(Pixels(8.0));
        })
        .height(Pixels(20.0))
        .alignment(Alignment::Center);

        let tabs = vec![
            TabDefinition::new("wave", "Waveform").with_width(80.0),
            TabDefinition::new("unison", "Unison").with_width(80.0),
        ];
        TabSwitcher::new(cx, tabs, move |cx, id, _| match id {
            "wave" => {
                HStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "SHAPE").class("knob-label");
                        waveform_dropdown(cx, Data::params, move |p| &osc(p).waveform);
                    })
                    .class("knob-stack");
                    octave_counter(cx, Data::params, move |p| &osc(p).octave);
                    param_knob_block(cx, "Freq", Data::params, move |p| &osc(p).frequency);
                    param_knob_block(cx, "Detune", Data::params, move |p| &osc(p).detune);
                    param_knob_block(cx, "Phase", Data::params, move |p| &osc(p).phase);
                    param_knob_block(cx, "Level", Data::params, move |p| &osc(p).gain);
                })
                .class("gap-16")
                .alignment(Alignment::Center);
            }
            "unison" => {
                HStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "VOICES").class("knob-label");
                        ParamKnob::new(cx, Data::params, move |p| &osc(p).unison_voices)
                            .size(Pixels(40.0));
                    })
                    .class("knob-stack");
                    param_knob_block(cx, "Detune", Data::params, move |p| &osc(p).unison_detune);
                    param_knob_block(cx, "Blend", Data::params, move |p| &osc(p).unison_blend);
                    param_knob_block(cx, "Gain", Data::params, move |p| &osc(p).unison_volume);
                })
                .class("gap-16")
                .alignment(Alignment::Center);
            }
            _ => {}
        })
        .height(Pixels(90.0));
    })
    .class("module-card");
}

pub(crate) fn create(
    params: Arc<SineParams>,
    editor_state: Arc<ViziaState>,
    // Reserved for the (currently inert) AI assist panel; see `crate::ai`.
    _mcp_state: Arc<RwLock<McpPluginState>>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // Register every stylesheet once here rather than per-widget-construction.
        cx.add_stylesheet(UI_STYLESHEET)
            .expect("Failed to load styles");
        cx.add_stylesheet(crate::knob::KNOB_CSS).ok();
        cx.add_stylesheet(crate::tab_switcher::TABSWITCHER_THEME).ok();

        Data {
            params: params.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            // Header with balanced alignment
            HStack::new(cx, |cx| {
                Label::new(cx, "TONEMORPH").class("app-title");
                Label::new(cx, "v1.0.0")
                    .font_size(10.0)
                    .color(ColorPalette::TEXT_LOW);
            })
            .class("header");

            let main_tabs = vec![
                TabDefinition::new("oscillators", "OSCILLATORS"),
                TabDefinition::new("envelope", "ENVELOPE"),
                TabDefinition::new("filters_fx", "FILTER & FX"),
                TabDefinition::new("ai", "AI ASSIST"),
            ];

            TabSwitcher::new(cx, main_tabs, move |cx, tab_id, _| {
                VStack::new(cx, |cx| match tab_id {
                    "oscillators" => {
                        VStack::new(cx, |cx| {
                            create_osc_section(
                                cx,
                                "Oscillator 1",
                                ColorPalette::OSC1_ACCENT,
                                |p| &p.osc1,
                            );
                            create_osc_section(
                                cx,
                                "Oscillator 2",
                                ColorPalette::OSC2_ACCENT,
                                |p| &p.osc2,
                            );
                            create_osc_section(
                                cx,
                                "Oscillator 3",
                                ColorPalette::OSC3_ACCENT,
                                |p| &p.osc3,
                            );
                        });
                    }
                    "filters_fx" => {
                        VStack::new(cx, |cx| {
                            VStack::new(cx, |cx| {
                                HStack::new(cx, |cx| {
                                    Element::new(cx)
                                        .width(Pixels(3.0))
                                        .height(Pixels(14.0))
                                        .background_color(ColorPalette::FILTER_ACCENT)
                                        .corner_radius(Pixels(1.5));
                                    Label::new(cx, "FILTER ENGINE")
                                        .class("module-title")
                                        .padding_left(Pixels(8.0));
                                })
                                .height(Pixels(20.0))
                                .alignment(Alignment::Center);
                                HStack::new(cx, |cx| {
                                    VStack::new(cx, |cx| {
                                        Label::new(cx, "MODE").class("knob-label");
                                        filter_mode_dropdown(cx, Data::params, |p| &p.filter.mode);
                                    })
                                    .class("knob-stack");
                                    param_knob_block(cx, "Cutoff", Data::params, |p| {
                                        &p.filter.cutoff
                                    });
                                    param_knob_block(cx, "Res", Data::params, |p| {
                                        &p.filter.resonance
                                    });
                                    param_knob_block(cx, "Drive", Data::params, |p| {
                                        &p.filter.drive
                                    });
                                })
                                .class("gap-16")
                                .alignment(Alignment::Center);
                            })
                            .class("module-card");
                            VStack::new(cx, |cx| {
                                Label::new(cx, "POST-PROCESS FX").class("module-title");
                                Element::new(cx)
                                    .height(Pixels(60.0))
                                    .background_color(ColorPalette::BG_CARD_ALT)
                                    .corner_radius(Pixels(4.0));
                            })
                            .class("module-card");
                        });
                    }
                    "envelope" => {
                        VStack::new(cx, |cx| {
                            Label::new(cx, "AMPLITUDE ENVELOPE").class("module-title");
                            HStack::new(cx, |cx| {
                                param_knob_block(cx, "Attack", Data::params, |p| &p.adsr.attack);
                                param_knob_block(cx, "Decay", Data::params, |p| &p.adsr.decay);
                                param_knob_block(cx, "Sustain", Data::params, |p| &p.adsr.sustain);
                                param_knob_block(cx, "Release", Data::params, |p| &p.adsr.release);
                            })
                            .class("gap-16");
                        })
                        .class("module-card");
                    }
                    "ai" => {
                        // AI assist panel is not wired up yet; see `crate::ai`.
                        // crate::ai::chat_ui::chat_panel(cx, _mcp_state.clone());
                    }
                    _ => {}
                })
                .class("padding-20");
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        })
        .class("root");
    })
}
