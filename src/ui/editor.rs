use super::{Meter, ParamKnob, PeakMeter, TabDefinition, TabSwitcher};
use crate::{FilterMode, OscillatorParams, SineParams, Waveform};
use nih_plug::prelude::{Editor, EnumParam, Param};
use std::sync::Arc;
use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::param_base::ParamWidgetBase;
use vizia_plug::widgets::*;
use vizia_plug::{create_vizia_editor, ViziaState, ViziaTheming};

// --- MODERN COLOR PALETTE ---
struct ColorPalette;
impl ColorPalette {
    pub const OSC1_ACCENT: Color = Color::rgb(56, 189, 248); // Cyan
    pub const OSC2_ACCENT: Color = Color::rgb(34, 197, 94); // Emerald
    pub const OSC3_ACCENT: Color = Color::rgb(244, 63, 94); // Rose
    pub const FILTER_ACCENT: Color = Color::rgb(168, 85, 247); // Purple
    pub const BG_CARD_ALT: Color = Color::rgb(28, 28, 34);
    pub const BORDER: Color = Color::rgb(45, 45, 52);
    pub const TEXT_HIGH: Color = Color::rgb(248, 250, 252);
    pub const TEXT_MED: Color = Color::rgb(148, 163, 184);
}

/// Per-oscillator knob accent classes (defined in `knob::KNOB_CSS`).
const ACCENT_OSC1: &str = "accent-cyan";
const ACCENT_OSC2: &str = "accent-emerald";
const ACCENT_OSC3: &str = "accent-rose";
const ACCENT_FILTER: &str = "accent-purple";
const ACCENT_DEFAULT: &str = "accent-indigo";

#[derive(Lens)]
struct Data {
    params: Arc<SineParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (760, 740))
}

// --- MODERN STYLESHEET ---
//
// Only property names this `vizia_style` revision actually parses are used here
// (`gap`, `padding`, `corner-radius`, `border`, `font-weight`, `transition`,
// `alignment`, …). Legacy names like `child-space`/`col-between`/`border-radius`
// are silently dropped by the parser, so they're deliberately avoided — spacing
// that must be reliable is set in Rust via `.gap()`/`.padding()` instead.
const UI_STYLESHEET: &str = r#"
    .root {
        background-color: #0A0A0C;
    }

    /* ---- Header ---- */
    .header {
        height: 56px;
        background-color: #121216;
        border-width: 0px 0px 1px 0px;
        border-color: #26262E;
        padding-left: 18px;
        padding-right: 18px;
        gap: 10px;
        alignment: center;
    }
    .app-title {
        color: #F8FAFC;
        font-weight: 800;
        font-size: 17px;
    }
    .app-subtitle {
        color: #6366F1;
        font-weight: 700;
        font-size: 9px;
    }
    .app-version {
        color: #475569;
        font-size: 10px;
    }
    .meter-stack {
        gap: 4px;
        alignment: center;
        width: auto;
    }
    .meter-caption {
        color: #64748B;
        font-size: 8px;
        font-weight: 700;
    }

    /* ---- Module cards ---- */
    .module-card {
        background-color: #15151A;
        border: 1px solid #26262E;
        corner-radius: 10px;
        padding: 16px;
        gap: 14px;
    }
    .module-head {
        height: 18px;
        gap: 8px;
        alignment: center;
    }
    .module-title {
        color: #F8FAFC;
        font-size: 11px;
        font-weight: 700;
    }

    /* ---- Knobs ---- */
    .knob-stack {
        alignment: center;
        gap: 6px;
        width: auto;
    }
    .knob-label {
        font-size: 9px;
        font-weight: 700;
        color: #64748B;
        text-align: center;
    }
    .knob-value {
        font-size: 9px;
        color: #94A3B8;
        text-align: center;
        width: 60px;
    }

    /* ---- Octave stepper ---- */
    .octave-counter {
        background-color: #0F141F;
        corner-radius: 6px;
        border: 1px solid #2E3340;
        overflow: hidden;
        alignment: center;
    }
    .counter-btn {
        width: 22px;
        height: 22px;
        background-color: transparent;
        color: #94A3B8;
        font-size: 14px;
        alignment: center;
        transition: background-color 120ms, color 120ms;
    }
    .counter-btn:hover {
        background-color: #1E293B;
        color: #F8FAFC;
    }
    .counter-value {
        width: 34px;
        color: #818CF8;
        font-weight: 700;
        font-size: 11px;
        text-align: center;
    }

    /* ---- Dropdowns ---- */
    .dropdown-trigger {
        background-color: #1C1C22;
        border: 1px solid #2E3340;
        corner-radius: 6px;
        alignment: center;
        transition: border-color 120ms;
    }
    .dropdown-trigger:hover {
        border-color: #6366F1;
    }
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
        .height(Pixels(24.0))
        .class("octave-counter");
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
                    .gap(Pixels(6.0))
                    .alignment(Alignment::Center)
                    .padding_left(Pixels(10.0))
                    .padding_right(Pixels(10.0))
                })
                .class("dropdown-trigger")
                .width(Pixels(96.0))
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
            .border_color(ColorPalette::BORDER)
            .corner_radius(Pixels(6.0));
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
                    .gap(Pixels(6.0))
                    .alignment(Alignment::Center)
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
                        let opt = option;
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
            .border_color(ColorPalette::BORDER)
            .corner_radius(Pixels(6.0));
        },
    )
    .placement(Placement::Bottom)
}

/// One labelled knob with a live value readout beneath it. Generic over the
/// parameter type, so the same cell drives `FloatParam` and `IntParam` knobs.
/// `accent` is the CSS class that tints the knob (e.g. `"accent-cyan"`).
fn knob_cell<L, P, FMap>(cx: &mut Context, label: &str, accent: &str, params: L, map: FMap)
where
    L: Lens<Target = Arc<SineParams>> + Clone + 'static + Send + Sync,
    P: Param + 'static,
    FMap: Fn(&Arc<SineParams>) -> &P + Copy + Send + Sync + 'static,
{
    VStack::new(cx, |cx| {
        Label::new(cx, label).class("knob-label");
        ParamKnob::new(cx, params.clone(), map)
            .size(Pixels(44.0))
            .class(accent);
        // Live, formatted value (e.g. "440 Hz", "-6.0 dB") — updates reactively
        // through a parameter lens, so host automation moves the text too.
        Label::new(
            cx,
            ParamWidgetBase::make_lens(params, map, |p| {
                p.normalized_value_to_string(p.modulated_normalized_value(), true)
            }),
        )
        .class("knob-value");
    })
    .class("knob-stack");
}

/// A small accent bar + uppercase title used as a module header.
fn module_header(cx: &mut Context, title: &str, accent: Color) {
    HStack::new(cx, |cx| {
        Element::new(cx)
            .width(Pixels(3.0))
            .height(Pixels(14.0))
            .background_color(accent)
            .corner_radius(Pixels(1.5));
        Label::new(cx, title).class("module-title");
    })
    .class("module-head");
}

/// Builds one oscillator module card. `osc` selects which of the three
/// oscillator param groups this section drives; every knob is derived from it,
/// so the three call sites differ only by selector and accent colour.
fn create_osc_section(
    cx: &mut Context,
    title: &str,
    accent: Color,
    accent_class: &'static str,
    osc: impl Fn(&SineParams) -> &OscillatorParams + Copy + Send + Sync + 'static,
) {
    VStack::new(cx, |cx| {
        module_header(cx, title, accent);

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
                    knob_cell(cx, "FREQ", accent_class, Data::params, move |p| {
                        &osc(p).frequency
                    });
                    knob_cell(cx, "DETUNE", accent_class, Data::params, move |p| {
                        &osc(p).detune
                    });
                    knob_cell(cx, "PHASE", accent_class, Data::params, move |p| {
                        &osc(p).phase
                    });
                    knob_cell(cx, "LEVEL", accent_class, Data::params, move |p| {
                        &osc(p).gain
                    });
                })
                .gap(Pixels(16.0))
                .alignment(Alignment::Center);
            }
            "unison" => {
                HStack::new(cx, |cx| {
                    knob_cell(cx, "VOICES", accent_class, Data::params, move |p| {
                        &osc(p).unison_voices
                    });
                    knob_cell(cx, "DETUNE", accent_class, Data::params, move |p| {
                        &osc(p).unison_detune
                    });
                    knob_cell(cx, "BLEND", accent_class, Data::params, move |p| {
                        &osc(p).unison_blend
                    });
                    knob_cell(cx, "GAIN", accent_class, Data::params, move |p| {
                        &osc(p).unison_volume
                    });
                })
                .gap(Pixels(16.0))
                .alignment(Alignment::Center);
            }
            _ => {}
        })
        .height(Pixels(96.0));
    })
    .class("module-card");
}

pub(crate) fn create(
    params: Arc<SineParams>,
    peak: Arc<PeakMeter>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // Register every stylesheet once here rather than per-widget-construction.
        cx.add_stylesheet(UI_STYLESHEET)
            .expect("Failed to load styles");
        cx.add_stylesheet(super::knob::KNOB_CSS).ok();
        cx.add_stylesheet(super::meter::METER_CSS).ok();
        cx.add_stylesheet(super::tab_switcher::TABSWITCHER_THEME).ok();
        cx.add_stylesheet(crate::ai::chat_ui::CHAT_STYLES).ok();

        Data {
            params: params.clone(),
        }
        .build(cx);

        // The AI tab's tools drive the live parameters directly.
        let ai_params = params.clone();
        let meter = peak.clone();

        VStack::new(cx, move |cx| {
            // Header: title block, flexible spacer, live output meter, version.
            HStack::new(cx, move |cx| {
                Label::new(cx, "TONEMORPH").class("app-title");
                Label::new(cx, "POLY SYNTH").class("app-subtitle");

                // Flexible spacer pushes the meter/version to the right edge.
                Element::new(cx).width(Stretch(1.0)).height(Pixels(0.0));

                VStack::new(cx, move |cx| {
                    Label::new(cx, "OUTPUT").class("meter-caption");
                    Meter::new(cx, meter.clone());
                })
                .class("meter-stack");

                Label::new(cx, "v1.0.0").class("app-version");
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
                                "OSCILLATOR 1",
                                ColorPalette::OSC1_ACCENT,
                                ACCENT_OSC1,
                                |p| &p.osc1,
                            );
                            create_osc_section(
                                cx,
                                "OSCILLATOR 2",
                                ColorPalette::OSC2_ACCENT,
                                ACCENT_OSC2,
                                |p| &p.osc2,
                            );
                            create_osc_section(
                                cx,
                                "OSCILLATOR 3",
                                ColorPalette::OSC3_ACCENT,
                                ACCENT_OSC3,
                                |p| &p.osc3,
                            );
                        })
                        .gap(Pixels(12.0));
                    }
                    "filters_fx" => {
                        VStack::new(cx, |cx| {
                            VStack::new(cx, |cx| {
                                module_header(cx, "FILTER ENGINE", ColorPalette::FILTER_ACCENT);
                                HStack::new(cx, |cx| {
                                    VStack::new(cx, |cx| {
                                        Label::new(cx, "MODE").class("knob-label");
                                        filter_mode_dropdown(cx, Data::params, |p| &p.filter.mode);
                                    })
                                    .class("knob-stack");
                                    knob_cell(cx, "CUTOFF", ACCENT_FILTER, Data::params, |p| {
                                        &p.filter.cutoff
                                    });
                                    knob_cell(cx, "RES", ACCENT_FILTER, Data::params, |p| {
                                        &p.filter.resonance
                                    });
                                    knob_cell(cx, "DRIVE", ACCENT_FILTER, Data::params, |p| {
                                        &p.filter.drive
                                    });
                                })
                                .gap(Pixels(16.0))
                                .alignment(Alignment::Center);
                            })
                            .class("module-card");

                            VStack::new(cx, |cx| {
                                module_header(cx, "POST-PROCESS FX", ColorPalette::FILTER_ACCENT);
                                Element::new(cx)
                                    .height(Pixels(60.0))
                                    .background_color(ColorPalette::BG_CARD_ALT)
                                    .corner_radius(Pixels(6.0));
                            })
                            .class("module-card");
                        })
                        .gap(Pixels(12.0));
                    }
                    "envelope" => {
                        VStack::new(cx, |cx| {
                            module_header(cx, "AMPLITUDE ENVELOPE", ColorPalette::FILTER_ACCENT);
                            HStack::new(cx, |cx| {
                                knob_cell(cx, "ATTACK", ACCENT_DEFAULT, Data::params, |p| {
                                    &p.adsr.attack
                                });
                                knob_cell(cx, "DECAY", ACCENT_DEFAULT, Data::params, |p| {
                                    &p.adsr.decay
                                });
                                knob_cell(cx, "SUSTAIN", ACCENT_DEFAULT, Data::params, |p| {
                                    &p.adsr.sustain
                                });
                                knob_cell(cx, "RELEASE", ACCENT_DEFAULT, Data::params, |p| {
                                    &p.adsr.release
                                });
                            })
                            .gap(Pixels(16.0))
                            .alignment(Alignment::Center);
                        })
                        .class("module-card");
                    }
                    "ai" => {
                        crate::ai::chat_ui::chat_panel(cx, ai_params.clone());
                    }
                    _ => {}
                })
                .padding(Pixels(20.0));
            })
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        })
        .class("root");
    })
}
