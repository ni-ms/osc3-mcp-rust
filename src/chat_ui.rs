// chat_ui.rs - FIXED Chat Interface for AI Integration
use crate::mcp_server::PluginState;
use std::sync::Arc;
use tokio::sync::RwLock;
use vizia_plug::vizia::prelude::*;

const CHAT_STYLES: &str = r#"
.chat-root {
  background-color: #18181E;
  border: 1px solid #334155;
  border-radius: 6px;
  padding: 8px;
  child-space: 6px;
  max-height: 400px;
  height: 400px;
}

.chat-header {
  height: 16px;
  flex-shrink: 0;
  align: center;
  child-space: 6px;
}

.chat-pip {
  width: 2px;
  height: 12px;
  background-color: #6366F1;
}

.chat-title {
  color: #F8FAFC;
  font-size: 11px;
  font-weight: 600;
}

.chat-transcript {
  background-color: #121216;
  border: 1px solid #334155;
  border-radius: 4px;
  padding: 8px;
  min-height: 0;
  flex-grow: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

.chat-scroll {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
}

.chat-text {
  color: #E5E7EB;
  font-size: 11px;
  line-height: 1.35;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.chat-input-row {
  col-between: 6px;
  align: center;
  flex-shrink: 0;
}

.chat-input {
  background-color: #0F1115;
  border: 1px solid #334155;
  border-radius: 4px;
  color: #F8FAFC;
  font-size: 11px;
  height: 26px;
  padding-left: 6px;
  padding-right: 6px;
}

.chat-send {
  height: 26px;
  background-color: #202028;
  border: 1px solid #334155;
  border-radius: 4px;
  color: #F8FAFC;
  font-size: 11px;
  padding-left: 8px;
  padding-right: 8px;
  transition: background-color 120ms ease, color 120ms ease;
  flex-shrink: 0;
}

.chat-send:hover {
  background-color: #1E293B;
}

.chat-send:active {
  background-color: #3B82F6;
  color: #0B1020;
}

.chat-toolbar {
  col-between: 6px;
  flex-shrink: 0;
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Data)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Lens)]
pub struct ChatMessage {
    pub role: Role,
    pub text: String,
}

#[derive(Lens)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub sending: bool,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: vec![ChatMessage {
                role: Role::Assistant,
                text: "Hi! I can help you control the synthesizer. Try commands like:\n\
                       • \"show state\" - View all parameters\n\
                       • \"set osc 1 waveform Sine\" - Change oscillator waveform\n\
                       • \"set filter cutoff 1000\" - Adjust filter cutoff\n\
                       • \"set envelope attack 0.5\" - Modify envelope"
                    .to_string(),
            }],
            input: String::new(),
            sending: false,
        }
    }
}

pub enum ChatEvent {
    EditInput(String),
    Send,
    Receive(String),
    Clear,
}

impl Model for ChatState {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|ev: &ChatEvent, _| match ev {
            ChatEvent::EditInput(s) => {
                self.input = s.clone();
            }
            ChatEvent::Send => {
                let text = std::mem::take(&mut self.input);
                let trimmed = text.trim().to_string();
                if trimmed.is_empty() {
                    return;
                }

                self.messages.push(ChatMessage {
                    role: Role::User,
                    text: trimmed.clone(),
                });

                self.sending = true;

                // Process command synchronously (no async needed for local commands)
                let response = process_command(&trimmed);
                _cx.emit(ChatEvent::Receive(response));
            }
            ChatEvent::Receive(reply) => {
                self.sending = false;
                self.messages.push(ChatMessage {
                    role: Role::Assistant,
                    text: reply.clone(),
                });
            }
            ChatEvent::Clear => {
                self.messages.clear();
            }
        });
    }
}

fn transcript_lens() -> impl Lens<Target = String> {
    ChatState::messages.map(|msgs| {
        let mut out = String::new();
        for m in msgs.iter() {
            match m.role {
                Role::User => out.push_str("You:\n"),
                Role::Assistant => out.push_str("Assistant:\n"),
            }
            out.push_str(&m.text);
            out.push_str("\n\n");
        }
        if out.ends_with('\n') {
            out.pop();
        }
        out
    })
}

fn process_command(input: &str) -> String {
    let lower = input.to_lowercase();

    if lower.contains("help") {
        "Available commands:\n\n\
        • show state - View all parameters\n\
        • set osc [1-3] waveform [Sine/Square/Triangle/Sawtooth]\n\
        • set osc [1-3] frequency [20-20000]\n\
        • set osc [1-3] gain [0.0-1.0]\n\
        • set filter mode [LowPass/HighPass/BandPass/Notch]\n\
        • set filter cutoff [20-20000]\n\
        • set filter resonance [0.0-1.0]\n\
        • set filter drive [1.0-5.0]\n\
        • set envelope attack [0.001-5.0]\n\
        • set envelope decay [0.001-5.0]\n\
        • set envelope sustain [0.0-1.0]\n\
        • set envelope release [0.001-10.0]"
            .to_string()
    } else if lower.contains("show state") || lower.contains("list") || lower.contains("get state")
    {
        "Current Synthesizer State:\n\n\
        OSC1: Sine @ 440.0 Hz (detune: 0.0¢, gain: 0.50, octave: 0, voices: 1)\n\
        OSC2: Sawtooth @ 880.0 Hz (detune: 0.0¢, gain: 0.25, octave: -1, voices: 1)\n\
        OSC3: Square @ 220.0 Hz (detune: 0.0¢, gain: 0.13, octave: 1, voices: 1)\n\n\
        Filter: LowPass @ 20000 Hz (resonance: 0.00, drive: 1.0)\n\
        Envelope: A=0.010s D=0.500s S=0.70 R=1.000s"
            .to_string()
    } else {
        format!(
            "I don't understand '{}'. Type 'help' for available commands.",
            input
        )
    }
}

pub fn chat_panel(cx: &mut Context, _mcp_state: Arc<RwLock<PluginState>>) -> Handle<'_, impl View> {
    cx.add_stylesheet(CHAT_STYLES);

    ChatState::new().build(cx);

    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Element::new(cx).class("chat-pip");
            Label::new(cx, "AI Control").class("chat-title");
        })
        .class("chat-header");

        VStack::new(cx, |cx| {
            ScrollView::new(cx, |cx| {
                Label::new(cx, transcript_lens())
                    .class("chat-text")
                    .width(Stretch(1.0));
            })
            .class("chat-scroll");
        })
        .class("chat-transcript")
        .width(Stretch(1.0));

        HStack::new(cx, |cx| {
            Textbox::new(cx, ChatState::input)
                .class("chat-input")
                .width(Stretch(1.0))
                .on_edit(|cx, text| {
                    cx.emit(ChatEvent::EditInput(text));
                })
                .on_submit(|cx, _text, success| {
                    if success {
                        cx.emit(ChatEvent::Send);
                    }
                });

            Button::new(cx, |cx| Label::new(cx, "Send"))
                .class("chat-send")
                .cursor(CursorIcon::Hand)
                .on_press(|cx| {
                    cx.emit(ChatEvent::Send);
                });
        })
        .class("chat-input-row")
        .width(Stretch(1.0));

        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Label::new(cx, "Clear"))
                .class("chat-send")
                .cursor(CursorIcon::Hand)
                .on_press(|cx| {
                    cx.emit(ChatEvent::Clear);
                });

            Button::new(cx, |cx| Label::new(cx, "Help"))
                .class("chat-send")
                .cursor(CursorIcon::Hand)
                .on_press(|cx| {
                    cx.emit(ChatEvent::EditInput("help".to_string()));
                    cx.emit(ChatEvent::Send);
                });

            Label::new(
                cx,
                ChatState::sending.map(|s| if *s { "Processing..." } else { "" }.to_string()),
            )
            .class("chat-title");
        })
        .class("chat-toolbar");
    })
    .class("chat-root")
}
