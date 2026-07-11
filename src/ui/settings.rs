//! Global AI settings: the gear button shown in the app header and the modal
//! overlay it toggles. This is the single place to set the OpenRouter API key
//! and model; both the AI Assist chat and the Audio Copy "AI Match" load the
//! shared [`AiConfig`] from disk on demand, so changes here apply everywhere.

use vizia_plug::vizia::prelude::*;

use crate::ai::llm::{AiConfig, SUGGESTED_MODELS};

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
        width: 420px;
        background-color: #15151A;
        border: 1px solid #2D2D34;
        corner-radius: 10px;
        padding: 18px;
        gap: 10px;
    }
    .settings-titlerow { height: 22px; gap: 8px; alignment: center; }
    .settings-label { color: #F8FAFC; font-size: 13px; font-weight: 700; width: 1s; }
    .settings-sublabel { color: #94A3B8; font-size: 10px; font-weight: 600; }
    .settings-hint { color: #64748B; font-size: 9px; }
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

pub enum SettingsEvent {
    /// Toggle the overlay open/closed (the gear button).
    Toggle,
    /// Force the overlay closed (Done / ✕ / backdrop).
    Close,
    SetApiKey(String),
    SetModel(String),
}

#[derive(Lens)]
pub struct SettingsState {
    pub is_open: bool,
    pub api_key: String,
    pub model: String,
    /// Held only so a settings write round-trips temperature unchanged.
    temperature: f32,
}

impl SettingsState {
    fn persist(&self) {
        let cfg = AiConfig {
            api_key: self.api_key.clone(),
            model: self.model.clone(),
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
            SettingsEvent::SetApiKey(k) => {
                // Trim pasted whitespace/newlines so the saved key is usable as-is.
                self.api_key = k.trim().to_string();
                self.persist();
            }
            SettingsEvent::SetModel(m) => {
                self.model = m.trim().to_string();
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
        api_key: cfg.api_key,
        model: cfg.model,
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
                    Label::new(cx, "AI SETTINGS").class("settings-label");
                    Button::new(cx, |cx| Label::new(cx, "✕"))
                        .on_press(|cx| cx.emit(SettingsEvent::Close))
                        .class("settings-close");
                })
                .class("settings-titlerow");

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
                            .toggle_class(
                                "selected",
                                SettingsState::model.map(move |sel| sel == slug),
                            );
                    }
                })
                .class("settings-models");
                Label::new(cx, "Any OpenRouter slug, e.g. openai/gpt-4o").class("settings-hint");

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
