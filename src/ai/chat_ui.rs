//! The "AI ASSIST" tab: a chat panel that drives the synth via the OpenRouter
//! tool-calling loop in [`super::llm`]. Parameter writes reach the real
//! `nih_plug` params through `RawParamEvent`s emitted from the background task.
//!
//! The API key / model live in the global settings overlay (the header gear);
//! this panel loads the shared [`AiConfig`] from disk each time it sends.

use crate::SineParams;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use vizia_plug::vizia::prelude::*;

use super::llm::AiConfig;

// NOTE: this `vizia_style` revision silently drops legacy spacing names like
// `row-between`/`col-between`/`border-radius` (see editor.rs). Use `gap` for
// stack spacing and `corner-radius` for rounded corners, or layout collapses.
pub const CHAT_STYLES: &str = r#"
    .chat-root {
        padding: 12px;
        gap: 8px;
        background-color: #0A0A0C;
    }
    .chat-header {
        height: 24px;
        gap: 8px;
        alignment: center;
    }
    .chat-title {
        color: #F8FAFC;
        font-size: 12px;
        font-weight: 700;
        width: 1s;
    }
    .chat-iconbtn {
        width: 28px;
        height: 24px;
        background-color: #1C1C22;
        border: 1px solid #2D2D34;
        corner-radius: 4px;
        color: #94A3B8;
        alignment: center;
    }
    .chat-iconbtn:hover { border-color: #6366F1; }
    .chat-transcript {
        height: 1s;
        background-color: #121216;
        border: 1px solid #2D2D34;
        corner-radius: 6px;
        padding: 8px;
    }
    .chat-msg { gap: 2px; padding-bottom: 8px; }
    .chat-role {
        font-size: 9px;
        font-weight: 700;
        color: #6366F1;
        text-transform: uppercase;
    }
    .chat-text {
        color: #E5E7EB;
        font-size: 11px;
        line-height: 1.4;
    }
    .chat-status {
        color: #94A3B8;
        font-size: 10px;
        height: 12px;
    }
    .chat-inputrow { height: 28px; gap: 6px; }
    .chat-input {
        background-color: #0F1115;
        border: 1px solid #2D2D34;
        corner-radius: 4px;
        color: #F8FAFC;
        font-size: 11px;
        height: 28px;
        padding-left: 8px;
    }
    .chat-input:focus-visible { border-color: #6366F1; }
    .chat-send {
        height: 28px;
        background-color: #6366F1;
        corner-radius: 4px;
        color: #F8FAFC;
        font-size: 11px;
        padding-left: 12px;
        padding-right: 12px;
        alignment: center;
    }
    .chat-send:hover { background-color: #818CF8; }
    .chat-stop {
        height: 28px;
        background-color: #3F1D2B;
        border: 1px solid #F43F5E;
        corner-radius: 4px;
        color: #FDA4AF;
        font-size: 11px;
        padding-left: 12px;
        padding-right: 12px;
        alignment: center;
    }
    .chat-stop:hover { background-color: #F43F5E; color: #0A0A0C; }
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Data)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

fn role_label(role: Role) -> &'static str {
    match role {
        Role::User => "You",
        Role::Assistant => "AI",
        Role::Tool => "•",
    }
}

#[derive(Clone, Data)]
pub struct ChatMessage {
    pub role: Role,
    pub text: String,
}

/// The opening assistant message, shown on launch and after "Clear".
fn greeting() -> ChatMessage {
    ChatMessage {
        role: Role::Assistant,
        text: "Describe a sound and I'll dial it in — e.g. \"warm detuned pad\" — \
               or ask me to save/load a preset. Set your OpenRouter API key in the \
               settings gear (⚙, top-right) first."
            .to_string(),
    }
}

pub enum ChatEvent {
    EditInput(String),
    Send,
    Receive(String),
    ToolLog(String),
    Status(String),
    /// Cancel the in-flight request.
    Stop,
    /// Reset the transcript to the opening message.
    Clear,
}

#[derive(Lens)]
pub struct ChatState {
    messages: Vec<ChatMessage>,
    input: String,
    sending: bool,
    status: String,
    params: Arc<SineParams>,
    /// Shared async runtime, built once when the panel opens. Each send drives a
    /// request on it via `block_on` from a `cx.spawn` thread, instead of standing
    /// up a fresh runtime (and thread pool) per message.
    runtime: Option<Arc<tokio::runtime::Runtime>>,
    /// Set to `true` by `Stop`/`Clear` to abort the in-flight agentic loop; the
    /// background task polls this between tool-call rounds. Reset on each `Send`.
    cancel: Arc<AtomicBool>,
}

impl Model for ChatState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|ev: &ChatEvent, _meta| match ev {
            ChatEvent::EditInput(s) => self.input = s.clone(),

            ChatEvent::Status(s) => self.status = s.clone(),

            ChatEvent::Stop => {
                // Signal the background loop to bail, then free the UI now so the
                // user can type again without waiting for the in-flight round.
                self.cancel.store(true, Ordering::Relaxed);
                self.sending = false;
                self.status.clear();
                self.messages.push(ChatMessage {
                    role: Role::Tool,
                    text: "⏹ Stopped.".to_string(),
                });
            }

            ChatEvent::Clear => {
                // Abort anything in flight and reset the transcript.
                self.cancel.store(true, Ordering::Relaxed);
                self.sending = false;
                self.status.clear();
                self.messages = vec![greeting()];
            }

            ChatEvent::Receive(text) => {
                self.sending = false;
                self.status.clear();
                self.messages.push(ChatMessage {
                    role: Role::Assistant,
                    text: text.clone(),
                });
            }

            ChatEvent::ToolLog(text) => {
                self.messages.push(ChatMessage {
                    role: Role::Tool,
                    text: text.clone(),
                });
            }

            ChatEvent::Send => {
                let text = self.input.trim().to_string();
                if text.is_empty() || self.sending {
                    return;
                }
                self.input.clear();
                self.messages.push(ChatMessage {
                    role: Role::User,
                    text: text.clone(),
                });

                // Pull the latest key/model from the shared settings on disk.
                let cfg = AiConfig::load();
                if cfg.api_key.trim().is_empty() {
                    self.messages.push(ChatMessage {
                        role: Role::Assistant,
                        text: "Set your OpenRouter API key in the settings gear (⚙, top-right) first."
                            .to_string(),
                    });
                    return;
                }

                let Some(rt) = self.runtime.clone() else {
                    self.messages.push(ChatMessage {
                        role: Role::Assistant,
                        text: "Async runtime is unavailable; cannot reach the AI service."
                            .to_string(),
                    });
                    return;
                };

                self.sending = true;
                self.status = "Thinking…".to_string();
                // Fresh run: clear any stale Stop from a previous request.
                self.cancel.store(false, Ordering::Relaxed);

                let params = self.params.clone();
                let convo: Vec<(Role, String)> =
                    self.messages.iter().map(|m| (m.role, m.text.clone())).collect();
                let cancel = self.cancel.clone();

                cx.spawn(move |proxy| {
                    rt.block_on(super::llm::run_conversation(
                        proxy, &params, &cfg, convo, cancel,
                    ));
                });
            }
        });
    }
}

/// Build the AI chat panel. `params` is the live parameter set the tools drive.
pub fn chat_panel(cx: &mut Context, params: Arc<SineParams>) {
    ChatState {
        messages: vec![greeting()],
        input: String::new(),
        sending: false,
        status: String::new(),
        params,
        runtime: tokio::runtime::Runtime::new().ok().map(Arc::new),
        cancel: Arc::new(AtomicBool::new(false)),
    }
    .build(cx);

    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, "AI SYNTH AGENT").class("chat-title");
            Button::new(cx, |cx| Label::new(cx, "Clear"))
                .on_press(|cx| cx.emit(ChatEvent::Clear))
                .class("chat-iconbtn")
                .width(Pixels(44.0));
        })
        .class("chat-header");

        let transcript = ScrollView::new(cx, |cx| {
            List::new(cx, ChatState::messages, |cx, _, item| {
                VStack::new(cx, |cx| {
                    Label::new(cx, item.map(|m| role_label(m.role).to_string())).class("chat-role");
                    Label::new(cx, item.map(|m| m.text.clone()))
                        .class("chat-text")
                        .width(Stretch(1.0));
                })
                .class("chat-msg");
            });
        })
        .class("chat-transcript")
        .entity();

        // Auto-scroll the transcript to the newest message. Each `Send`,
        // `ToolLog`, and `Receive` pushes onto `messages`, so binding to its
        // length pins the view to the bottom as the agent streams tool logs and
        // its final reply. `SetY(1.0)` is normalized progress, so it lands at the
        // bottom regardless of the (post-layout) content height.
        Binding::new(cx, ChatState::messages.map(|m| m.len()), move |cx, _len| {
            cx.emit_to(transcript, ScrollEvent::SetY(1.0));
        });

        Label::new(cx, ChatState::status).class("chat-status");

        HStack::new(cx, |cx| {
            Textbox::new(cx, ChatState::input)
                .class("chat-input")
                .width(Stretch(1.0))
                .on_edit(|cx, text| cx.emit(ChatEvent::EditInput(text)))
                .on_submit(|cx, _, _| cx.emit(ChatEvent::Send));
            // While a request is in flight the button becomes a Stop control.
            Binding::new(cx, ChatState::sending, |cx, sending| {
                if sending.get(cx) {
                    Button::new(cx, |cx| Label::new(cx, "Stop"))
                        .on_press(|cx| cx.emit(ChatEvent::Stop))
                        .class("chat-stop");
                } else {
                    Button::new(cx, |cx| Label::new(cx, "Send"))
                        .on_press(|cx| cx.emit(ChatEvent::Send))
                        .class("chat-send");
                }
            });
        })
        .class("chat-inputrow");
    })
    .class("chat-root");
}
