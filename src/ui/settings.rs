//! Global settings: the gear button shown in the app header and the modal
//! overlay it toggles. The overlay is organised as a left-hand tab rail with
//! three sections:
//!
//! - **AI** — pick the provider (OpenRouter or a local Ollama server) and its
//!   model/key. Both the AI Assist chat and the Audio Copy "AI Match" load the
//!   shared [`AiConfig`] from disk on demand, so changes here apply everywhere.
//! - **Color Scheme** — placeholder for future theming.
//! - **About** — version / creator / links.

use vizia_plug::vizia::prelude::*;

use crate::ai::llm::{AiConfig, AiProvider, SUGGESTED_MODELS, SUGGESTED_OLLAMA_MODELS};

pub const SETTINGS_CSS: &str = r#"
    .settings-gear {
        width: 30px;
        height: 30px;
        background-color: #1C1C22;
        border: 1px solid #2D2D34;
        corner-radius: 6px;
        color: #94A3B8;
        font-size: 14px;
        alignment: center;
        transition: border-color 120ms, color 120ms;
    }
    .settings-gear:hover { border-color: #6366F1; color: #F8FAFC; }

    .settings-overlay {
        position-type: absolute;
        width: 100%;
        height: 100%;
        left: 0px;
        top: 0px;
        background-color: #05050BE6;
        alignment: center;
    }
    .settings-card {
        width: 620px;
        background-color: #15151A;
        border: 1px solid #2D2D34;
        corner-radius: 10px;
        padding: 18px;
        gap: 14px;
    }
    .settings-titlerow { height: 22px; gap: 8px; alignment: center; }
    .settings-label { color: #F8FAFC; font-size: 13px; font-weight: 700; width: 1s; }
    .settings-sublabel { color: #94A3B8; font-size: 10px; font-weight: 600; }
    .settings-hint { color: #64748B; font-size: 9px; }

    /* ---- Body: left rail + content ---- */
    .settings-body { height: 320px; gap: 18px; }
    .settings-rail { width: 150px; gap: 4px; }
    .nav-item {
        height: 34px;
        background-color: transparent;
        border: 1px solid transparent;
        corner-radius: 6px;
        color: #94A3B8;
        font-size: 12px;
        font-weight: 600;
        padding-left: 12px;
        alignment: left;
        transition: background-color 120ms, color 120ms;
    }
    .nav-item:hover { background-color: #1C1C22; color: #F8FAFC; }
    .nav-item.active {
        background-color: #1C1C22;
        border-color: #6366F1;
        color: #F8FAFC;
    }
    .settings-content { width: 1s; gap: 10px; }
    .section-title { color: #F8FAFC; font-size: 13px; font-weight: 700; }

    .settings-input {
        background-color: #0F1115;
        border: 1px solid #2D2D34;
        corner-radius: 4px;
        color: #F8FAFC;
        font-size: 11px;
        height: 28px;
        padding-left: 8px;
    }
    .settings-input:focus-visible { border-color: #6366F1; }

    /* ---- Provider radio ---- */
    .radio-row {
        height: 40px;
        gap: 10px;
        alignment: left;
        padding-left: 12px;
        padding-right: 12px;
        background-color: #0F1115;
        border: 1px solid #2D2D34;
        corner-radius: 6px;
        width: 1s;
        transition: border-color 120ms;
    }
    .radio-row:hover { border-color: #45455060; }
    .radio-row.selected { border-color: #6366F1; background-color: #171826; }
    .radio-dot {
        width: 14px;
        height: 14px;
        corner-radius: 7px;
        border: 2px solid #475569;
        background-color: transparent;
    }
    .radio-dot.on { border-color: #6366F1; background-color: #6366F1; }
    .radio-title { color: #F8FAFC; font-size: 11px; font-weight: 700; }
    .radio-sub { color: #64748B; font-size: 9px; }

    /* ---- Model quick-pick chips ---- */
    .settings-models { gap: 6px; height: 26px; }
    .model-btn {
        background-color: #1C1C22;
        border: 1px solid #2D2D34;
        corner-radius: 4px;
        color: #94A3B8;
        font-size: 10px;
        padding-left: 10px;
        padding-right: 10px;
        alignment: center;
    }
    .model-btn:hover { border-color: #6366F1; }
    .model-btn.selected { background-color: #6366F1; color: #F8FAFC; }

    /* ---- About ---- */
    .about-name { color: #F8FAFC; font-size: 18px; font-weight: 800; }
    .about-tag { color: #6366F1; font-size: 10px; font-weight: 700; }
    .about-row { gap: 6px; alignment: left; height: 16px; }
    .about-key { color: #64748B; font-size: 10px; font-weight: 700; width: 70px; }
    .about-val { color: #CBD5E1; font-size: 10px; }
    .about-body { color: #94A3B8; font-size: 10px; line-height: 1.5; }

    .placeholder {
        color: #64748B;
        font-size: 11px;
        line-height: 1.5;
    }

    .settings-done {
        height: 28px;
        background-color: #6366F1;
        corner-radius: 4px;
        color: #F8FAFC;
        font-size: 11px;
        padding-left: 14px;
        padding-right: 14px;
        alignment: center;
    }
    .settings-done:hover { background-color: #818CF8; }
    .settings-close {
        width: 24px;
        height: 22px;
        background-color: transparent;
        color: #94A3B8;
        font-size: 13px;
        alignment: center;
    }
    .settings-close:hover { color: #F8FAFC; }
"#;

/// Which left-rail section is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Data)]
pub enum SettingsSection {
    Ai,
    Colors,
    About,
}

pub enum SettingsEvent {
    /// Toggle the overlay open/closed (the gear button).
    Toggle,
    /// Force the overlay closed (Done / ✕ / backdrop).
    Close,
    SelectSection(SettingsSection),
    SetProvider(AiProvider),
    SetApiKey(String),
    SetModel(String),
    SetOllamaUrl(String),
    SetOllamaModel(String),
}

#[derive(Lens)]
pub struct SettingsState {
    pub is_open: bool,
    pub section: SettingsSection,
    pub provider: AiProvider,
    pub api_key: String,
    pub model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    /// Held only so a settings write round-trips temperature unchanged.
    temperature: f32,
}

impl SettingsState {
    fn persist(&self) {
        let cfg = AiConfig {
            provider: self.provider,
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            ollama_url: self.ollama_url.clone(),
            ollama_model: self.ollama_model.clone(),
            temperature: self.temperature,
        };
        let _ = cfg.save();
    }
}

impl Model for SettingsState {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|ev: &SettingsEvent, _meta| match ev {
            SettingsEvent::Toggle => self.is_open = !self.is_open,
            SettingsEvent::Close => self.is_open = false,
            SettingsEvent::SelectSection(s) => self.section = *s,
            SettingsEvent::SetProvider(p) => {
                self.provider = *p;
                self.persist();
            }
            SettingsEvent::SetApiKey(k) => {
                // Trim pasted whitespace/newlines so the saved key is usable as-is.
                self.api_key = k.trim().to_string();
                self.persist();
            }
            SettingsEvent::SetModel(m) => {
                self.model = m.trim().to_string();
                self.persist();
            }
            SettingsEvent::SetOllamaUrl(u) => {
                self.ollama_url = u.trim().to_string();
                self.persist();
            }
            SettingsEvent::SetOllamaModel(m) => {
                self.ollama_model = m.trim().to_string();
                self.persist();
            }
        });
    }
}

/// Build the shared settings model on the current (root) context. Call once,
/// before placing [`settings_button`] / [`settings_overlay`].
pub fn build_settings_model(cx: &mut Context) {
    let cfg = AiConfig::load();
    SettingsState {
        is_open: false,
        section: SettingsSection::Ai,
        provider: cfg.provider,
        api_key: cfg.api_key,
        model: cfg.model,
        ollama_url: cfg.ollama_url,
        ollama_model: cfg.ollama_model,
        temperature: cfg.temperature,
    }
    .build(cx);
}

/// The gear button for the app header.
pub fn settings_button(cx: &mut Context) {
    Button::new(cx, |cx| Label::new(cx, "⚙"))
        .on_press(|cx| cx.emit(SettingsEvent::Toggle))
        .class("settings-gear");
}

/// One left-rail entry.
fn nav_item(cx: &mut Context, label: &str, section: SettingsSection) {
    Button::new(cx, |cx| Label::new(cx, label))
        .on_press(move |cx| cx.emit(SettingsEvent::SelectSection(section)))
        .class("nav-item")
        .width(Stretch(1.0))
        .cursor(CursorIcon::Hand)
        .toggle_class(
            "active",
            SettingsState::section.map(move |s| *s == section),
        );
}

/// One provider radio option (dot + title + subtitle), whole row clickable.
fn provider_radio(cx: &mut Context, title: &str, sub: &str, value: AiProvider) {
    HStack::new(cx, |cx| {
        Element::new(cx)
            .class("radio-dot")
            .toggle_class("on", SettingsState::provider.map(move |p| *p == value));
        VStack::new(cx, |cx| {
            Label::new(cx, title).class("radio-title");
            Label::new(cx, sub).class("radio-sub");
        })
        .gap(Pixels(1.0));
    })
    .class("radio-row")
    .cursor(CursorIcon::Hand)
    .toggle_class("selected", SettingsState::provider.map(move |p| *p == value))
    .on_press(move |cx| cx.emit(SettingsEvent::SetProvider(value)));
}

/// The AI section: provider radios + provider-specific fields.
fn ai_section(cx: &mut Context) {
    Label::new(cx, "AI PROVIDER").class("section-title");
    HStack::new(cx, |cx| {
        provider_radio(cx, "OpenRouter", "Cloud API · needs a key", AiProvider::OpenRouter);
        provider_radio(cx, "Ollama", "Local · runs on your machine", AiProvider::Ollama);
    })
    .gap(Pixels(10.0))
    .height(Pixels(40.0));

    // Provider-specific fields swap in place.
    Binding::new(cx, SettingsState::provider, |cx, provider| {
        match provider.get(cx) {
            AiProvider::OpenRouter => {
                Label::new(cx, "OpenRouter API Key").class("settings-sublabel");
                Textbox::new(cx, SettingsState::api_key)
                    .class("settings-input")
                    .width(Stretch(1.0))
                    .on_edit(|cx, k| cx.emit(SettingsEvent::SetApiKey(k)));
                Label::new(cx, "Create a key at openrouter.ai/keys").class("settings-hint");

                Label::new(cx, "Model").class("settings-sublabel");
                Textbox::new(cx, SettingsState::model)
                    .class("settings-input")
                    .width(Stretch(1.0))
                    .on_edit(|cx, m| cx.emit(SettingsEvent::SetModel(m)));
                HStack::new(cx, |cx| {
                    for (slug, label) in SUGGESTED_MODELS {
                        let slug = *slug;
                        Button::new(cx, move |cx| Label::new(cx, *label))
                            .on_press(move |cx| cx.emit(SettingsEvent::SetModel(slug.to_string())))
                            .class("model-btn")
                            .toggle_class("selected", SettingsState::model.map(move |s| s == slug));
                    }
                })
                .class("settings-models");
                Label::new(cx, "Any OpenRouter slug, e.g. openai/gpt-4o").class("settings-hint");
            }
            AiProvider::Ollama => {
                Label::new(cx, "Server URL").class("settings-sublabel");
                Textbox::new(cx, SettingsState::ollama_url)
                    .class("settings-input")
                    .width(Stretch(1.0))
                    .on_edit(|cx, u| cx.emit(SettingsEvent::SetOllamaUrl(u)));
                Label::new(cx, "Default http://localhost:11434 · start it with `ollama serve`")
                    .class("settings-hint");

                Label::new(cx, "Model").class("settings-sublabel");
                Textbox::new(cx, SettingsState::ollama_model)
                    .class("settings-input")
                    .width(Stretch(1.0))
                    .on_edit(|cx, m| cx.emit(SettingsEvent::SetOllamaModel(m)));
                HStack::new(cx, |cx| {
                    for (slug, label) in SUGGESTED_OLLAMA_MODELS {
                        let slug = *slug;
                        Button::new(cx, move |cx| Label::new(cx, *label))
                            .on_press(move |cx| {
                                cx.emit(SettingsEvent::SetOllamaModel(slug.to_string()))
                            })
                            .class("model-btn")
                            .toggle_class(
                                "selected",
                                SettingsState::ollama_model.map(move |s| s == slug),
                            );
                    }
                })
                .class("settings-models");
                Label::new(
                    cx,
                    "Pull it first (`ollama pull llama3.1`). Use a tool-calling model.",
                )
                .class("settings-hint");
            }
        }
    });
}

/// The About section.
fn about_section(cx: &mut Context) {
    Label::new(cx, "TONEMORPH").class("about-name");
    Label::new(cx, "POLYPHONIC TRIPLE-OSCILLATOR SYNTH").class("about-tag");

    VStack::new(cx, |cx| {
        about_row(cx, "Version", env!("CARGO_PKG_VERSION"));
        about_row(cx, "Creator", "Nikhil Sharma");
        about_row(cx, "Engine", "nih-plug · vizia · Rust 2024");
        about_row(cx, "Formats", "CLAP · VST3 · Standalone");
    })
    .gap(Pixels(6.0))
    .top(Pixels(6.0));

    Label::new(
        cx,
        "A subtractive synth with three oscillators, a multimode filter, dual ADSR \
         envelopes, and an AI sound-design assistant. Built on nih-plug with a vizia GUI.",
    )
    .class("about-body")
    .width(Stretch(1.0))
    .top(Pixels(8.0));
}

fn about_row(cx: &mut Context, key: &str, val: &str) {
    HStack::new(cx, |cx| {
        Label::new(cx, key).class("about-key");
        Label::new(cx, val).class("about-val");
    })
    .class("about-row");
}

/// The Color Scheme section — placeholder until theming lands.
fn colors_section(cx: &mut Context) {
    Label::new(cx, "COLOR SCHEME").class("section-title");
    Label::new(
        cx,
        "Theme customization is coming soon. You'll be able to pick an accent \
         color and light/dark surfaces here.",
    )
    .class("placeholder")
    .width(Stretch(1.0));
}

/// The modal settings overlay. Renders nothing while closed; place it last in
/// the root so it floats above the tabs when open.
pub fn settings_overlay(cx: &mut Context) {
    Binding::new(cx, SettingsState::is_open, |cx, open| {
        if !open.get(cx) {
            return;
        }
        // Full-window backdrop; clicking it (outside the card) closes settings.
        VStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "SETTINGS").class("settings-label");
                    Button::new(cx, |cx| Label::new(cx, "✕"))
                        .on_press(|cx| cx.emit(SettingsEvent::Close))
                        .class("settings-close");
                })
                .class("settings-titlerow");

                HStack::new(cx, |cx| {
                    // Left rail.
                    VStack::new(cx, |cx| {
                        nav_item(cx, "AI", SettingsSection::Ai);
                        nav_item(cx, "Color Scheme", SettingsSection::Colors);
                        nav_item(cx, "About", SettingsSection::About);
                    })
                    .class("settings-rail");

                    // Content swaps with the active section.
                    VStack::new(cx, |cx| {
                        Binding::new(cx, SettingsState::section, |cx, section| {
                            match section.get(cx) {
                                SettingsSection::Ai => ai_section(cx),
                                SettingsSection::Colors => colors_section(cx),
                                SettingsSection::About => about_section(cx),
                            }
                        });
                    })
                    .class("settings-content");
                })
                .class("settings-body");

                Button::new(cx, |cx| Label::new(cx, "Done"))
                    .on_press(|cx| cx.emit(SettingsEvent::Close))
                    .class("settings-done");
            })
            .class("settings-card")
            // Swallow clicks on the card so they don't reach the backdrop.
            .on_press(|_| {});
        })
        .class("settings-overlay")
        .on_press(|cx| cx.emit(SettingsEvent::Close));
    });
}
