use vizia_plug::vizia::prelude::*;
use vizia_plug::widgets::*;

const CHAT_STYLES: &str = r#"
.chat-root {
  background-color: #18181E;
  border: 1px solid #334155;
  border-radius: 6px;
  padding: 8px;
  child-space: 6px;
}

.chat-header {
  height: 16px;
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
}

.chat-text {
  color: #E5E7EB;
  font-size: 11px;
  line-height: 1.35;
  white-space: pre-wrap;
}

.chat-input-row {
  col-between: 6px;
  align: center;
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

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
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
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
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
                let reply = format!("Echo: {}", trimmed);

                cx.emit(ChatEvent::Receive(reply));
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
                Role::User => out.push_str("User:\n"),
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

pub fn chat_panel(cx: &mut Context) -> Handle<'_, impl View> {
    cx.add_stylesheet(CHAT_STYLES);

    ChatState::default().build(cx);

    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Element::new(cx).class("chat-pip");
            Label::new(cx, "Chatbot").class("chat-title");
        })
        .class("chat-header");

        VStack::new(cx, |cx| {
            ScrollView::new(cx, |cx| {
                Label::new(cx, transcript_lens())
                    .class("chat-text")
                    .width(Stretch(1.0));
            })
            .width(Stretch(1.0))
            .height(Pixels(260.0));
        })
        .class("chat-transcript")
        .width(Stretch(1.0));

        HStack::new(cx, |cx| {
            Textbox::new(cx, ChatState::input)
                .class("chat-input")
                .width(Stretch(1.0))
                .on_edit(|cx, text| cx.emit(ChatEvent::EditInput(text)))
                .on_submit(|cx, _text, success| {
                    if success {
                        cx.emit(ChatEvent::Send);
                    }
                });

            Button::new(cx, |cx| Label::new(cx, "Send"))
                .class("chat-send")
                .cursor(CursorIcon::Hand)
                .on_press(|cx| cx.emit(ChatEvent::Send));
        })
        .class("chat-input-row")
        .width(Stretch(1.0));

        HStack::new(cx, |cx| {
            Button::new(cx, |cx| Label::new(cx, "Clear"))
                .class("chat-send")
                .cursor(CursorIcon::Hand)
                .on_press(|cx| cx.emit(ChatEvent::Clear));

            Label::new(
                cx,
                ChatState::sending.map(|s| if *s { "Thinking..." } else { "" }.to_string()),
            )
            .class("chat-title");
        })
        .class("chat-toolbar");
    })
    .class("chat-root")
}
