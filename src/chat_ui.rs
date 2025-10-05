
use crate::mcp_server::PluginState as McpPluginState;
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
    #[lens(ignore)]
    pub mcp_state: Arc<RwLock<McpPluginState>>,
}

impl ChatState {
    pub fn new(mcp_state: Arc<RwLock<McpPluginState>>) -> Self {
        Self {
            messages: vec![ChatMessage {
                role: Role::Assistant,
                text: "Hi! I can help you control the synthesizer. Try commands like:\n\
                       - \"show state\" - View all parameters\n\
                       - \"set osc 1 waveform Sine\" - Change oscillator waveform\n\
                       - \"set filter cutoff 1000\" - Adjust filter cutoff\n\
                       - \"set envelope attack 0.5\" - Modify envelope"
                    .to_string(),
            }],
            input: String::new(),
            sending: false,
            mcp_state,
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

                
                let mcp_state = self.mcp_state.clone();
                let reply = process_command(&trimmed, mcp_state);

                cx.emit(ChatEvent::Receive(reply));
            }
            ChatEvent::Receive(reply) => {
                self.sending = false;
                self.messages.push(ChatMessage {
                    role: Role::Assistant,
                    text: *reply,
                });
            }
            ChatEvent::Clear => {
                self.messages.clear();
            }
        });
    }
}

fn process_command(input: &str, mcp_state: Arc<RwLock<McpPluginState>>) -> String {
    let lower = input.to_lowercase();
    let parts: Vec<&str> = input.split_whitespace().collect();

    
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        if lower.contains("show state") || lower.contains("list") || lower.contains("get state") {
            
            let state = mcp_state.read().await;
            format!(
                "Current Synthesizer State:\n\n\
                OSC1: {} @ {:.1} Hz (detune: {:.1}¢, gain: {:.2}, octave: {}, voices: {})\n\
                OSC2: {} @ {:.1} Hz (detune: {:.1}¢, gain: {:.2}, octave: {}, voices: {})\n\
                OSC3: {} @ {:.1} Hz (detune: {:.1}¢, gain: {:.2}, octave: {}, voices: {})\n\n\
                Filter: {} @ {:.0} Hz (resonance: {:.2}, drive: {:.1})\n\
                Envelope: A={:.3}s D={:.3}s S={:.2} R={:.3}s",
                state.osc1_waveform,
                state.osc1_frequency,
                state.osc1_detune,
                state.osc1_gain,
                state.osc1_octave,
                state.osc1_unison_voices,
                state.osc2_waveform,
                state.osc2_frequency,
                state.osc2_detune,
                state.osc2_gain,
                state.osc2_octave,
                state.osc2_unison_voices,
                state.osc3_waveform,
                state.osc3_frequency,
                state.osc3_detune,
                state.osc3_gain,
                state.osc3_octave,
                state.osc3_unison_voices,
                state.filter_mode,
                state.filter_cutoff,
                state.filter_resonance,
                state.filter_drive,
                state.envelope_attack,
                state.envelope_decay,
                state.envelope_sustain,
                state.envelope_release
            )
        } else if lower.starts_with("set osc") || lower.starts_with("osc") {
            
            
            let osc_idx = parts
                .iter()
                .position(|&p| p == "osc")
                .and_then(|i| parts.get(&(i + 1)))
                .and_then(|s| s.parse::<u8>().ok());

            if let Some(osc) = osc_idx {
                if osc < 1 || osc > 3 {
                    return "Oscillator must be 1, 2, or 3".to_string();
                }

                let mut state = mcp_state.write().await;

                
                if lower.contains("waveform") || lower.contains("wave") {
                    if let Some(wave) = parts.iter().find(|&&p| {
                        let pl = p.to_lowercase();
                        pl == "sine" || pl == "square" || pl == "triangle" || pl == "sawtooth"
                    }) {
                        let waveform = wave.to_string();
                        let cap_wave = format!(
                            "{}{}",
                            waveform.chars().next().unwrap().to_uppercase(),
                            &waveform[1..]
                        );

                        match osc {
                            1 => state.osc1_waveform = cap_wave.clone(),
                            2 => state.osc2_waveform = cap_wave.clone(),
                            3 => state.osc3_waveform = cap_wave.clone(),
                            _ => unreachable!(),
                        }
                        return format!("✓ Set oscillator {} waveform to {}", osc, cap_wave);
                    }
                }

                
                if lower.contains("freq") || lower.contains("frequency") {
                    if let Some(freq_str) = parts
                        .iter()
                        .skip_while(|&&p| !p.to_lowercase().contains("freq"))
                        .nth(1)
                        .and_then(|s| s.parse::<f32>().ok())
                    {
                        let clamped = freq_str.clamp(20.0, 20000.0);
                        match osc {
                            1 => state.osc1_frequency = clamped,
                            2 => state.osc2_frequency = clamped,
                            3 => state.osc3_frequency = clamped,
                            _ => unreachable!(),
                        }
                        return format!("✓ Set oscillator {} frequency to {:.1} Hz", osc, clamped);
                    }
                }

                
                if lower.contains("gain") || lower.contains("volume") {
                    if let Some(gain_str) = parts
                        .iter()
                        .skip_while(|&&p| {
                            let pl = p.to_lowercase();
                            !pl.contains("gain") && !pl.contains("volume")
                        })
                        .nth(1)
                        .and_then(|s| s.parse::<f32>().ok())
                    {
                        let clamped = gain_str.clamp(0.0, 1.0);
                        match osc {
                            1 => state.osc1_gain = clamped,
                            2 => state.osc2_gain = clamped,
                            3 => state.osc3_gain = clamped,
                            _ => unreachable!(),
                        }
                        return format!("✓ Set oscillator {} gain to {:.2}", osc, clamped);
                    }
                }

                
                if lower.contains("octave") {
                    if let Some(oct_str) = parts
                        .iter()
                        .skip_while(|&&p| !p.to_lowercase().contains("octave"))
                        .nth(1)
                        .and_then(|s| s.parse::<i32>().ok())
                    {
                        let clamped = oct_str.clamp(-4, 4);
                        match osc {
                            1 => state.osc1_octave = clamped,
                            2 => state.osc2_octave = clamped,
                            3 => state.osc3_octave = clamped,
                            _ => unreachable!(),
                        }
                        return format!("✓ Set oscillator {} octave to {}", osc, clamped);
                    }
                }

                "Specify a property: waveform, frequency, gain, or octave".to_string()
            } else {
                "Specify oscillator number (1, 2, or 3)".to_string()
            }
        } else if lower.starts_with("set filter") || lower.starts_with("filter") {
            let mut state = mcp_state.write().await;

            
            if lower.contains("mode") {
                if let Some(mode) = parts.iter().find(|&&p| {
                    let pl = p.to_lowercase();
                    pl == "lowpass" || pl == "highpass" || pl == "bandpass" || pl == "notch"
                }) {
                    let cap_mode = format!(
                        "{}{}",
                        mode.chars().next().unwrap().to_uppercase(),
                        &mode[1..]
                    );
                    state.filter_mode = cap_mode.clone();
                    return format!("✓ Set filter mode to {}", cap_mode);
                }
            }

            
            if lower.contains("cutoff") {
                if let Some(cutoff_str) = parts
                    .iter()
                    .skip_while(|&&p| !p.to_lowercase().contains("cutoff"))
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    let clamped = cutoff_str.clamp(20.0, 20000.0);
                    state.filter_cutoff = clamped;
                    return format!("✓ Set filter cutoff to {:.0} Hz", clamped);
                }
            }

            
            if lower.contains("resonance") || lower.contains("res") {
                if let Some(res_str) = parts
                    .iter()
                    .skip_while(|&&p| {
                        let pl = p.to_lowercase();
                        !pl.contains("resonance") && !pl.contains("res")
                    })
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    let clamped = res_str.clamp(0.0, 1.0);
                    state.filter_resonance = clamped;
                    return format!("✓ Set filter resonance to {:.2}", clamped);
                }
            }

            
            if lower.contains("drive") {
                if let Some(drive_str) = parts
                    .iter()
                    .skip_while(|&&p| !p.to_lowercase().contains("drive"))
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    let clamped = drive_str.clamp(1.0, 5.0);
                    state.filter_drive = clamped;
                    return format!("✓ Set filter drive to {:.1}", clamped);
                }
            }

            "Specify a property: mode, cutoff, resonance, or drive".to_string()
        } else if lower.starts_with("set env") || lower.contains("envelope") {
            let mut state = mcp_state.write().await;

            if lower.contains("attack") {
                if let Some(val) = parts
                    .iter()
                    .skip_while(|&&p| !p.to_lowercase().contains("attack"))
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    let clamped = val.clamp(0.001, 5.0);
                    state.envelope_attack = clamped;
                    return format!("✓ Set envelope attack to {:.3}s", clamped);
                }
            }

            if lower.contains("decay") {
                if let Some(val) = parts
                    .iter()
                    .skip_while(|&&p| !p.to_lowercase().contains("decay"))
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    let clamped = val.clamp(0.001, 5.0);
                    state.envelope_decay = clamped;
                    return format!("✓ Set envelope decay to {:.3}s", clamped);
                }
            }

            if lower.contains("sustain") {
                if let Some(val) = parts
                    .iter()
                    .skip_while(|&&p| !p.to_lowercase().contains("sustain"))
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    let clamped = val.clamp(0.0, 1.0);
                    state.envelope_sustain = clamped;
                    return format!("✓ Set envelope sustain to {:.2}", clamped);
                }
            }

            if lower.contains("release") {
                if let Some(val) = parts
                    .iter()
                    .skip_while(|&&p| !p.to_lowercase().contains("release"))
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok())
                {
                    let clamped = val.clamp(0.001, 10.0);
                    state.envelope_release = clamped;
                    return format!("✓ Set envelope release to {:.3}s", clamped);
                }
            }

            "Specify a property: attack, decay, sustain, or release".to_string()
        } else if lower.contains("help") {
            "Available commands:\n\n\
            • show state - View all parameters\n\
            • set osc [1-3] waveform [Sine/Square/Triangle/Sawtooth]\n\
            • set osc [1-3] frequency [20-20000]\n\
            • set osc [1-3] gain [0.0-1.0]\n\
            • set osc [1-3] octave [-4 to 4]\n\
            • set filter mode [LowPass/HighPass/BandPass/Notch]\n\
            • set filter cutoff [20-20000]\n\
            • set filter resonance [0.0-1.0]\n\
            • set filter drive [1.0-5.0]\n\
            • set envelope attack [0.001-5.0]\n\
            • set envelope decay [0.001-5.0]\n\
            • set envelope sustain [0.0-1.0]\n\
            • set envelope release [0.001-10.0]"
                .to_string()
        } else {
            format!(
                "I don't understand '{}'. Type 'help' for available commands.",
                input
            )
        }
    })
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

pub fn chat_panel(
    cx: &mut Context,
    mcp_state: Arc<RwLock<McpPluginState>>,
) -> Handle<'_, impl View> {
    cx.add_stylesheet(CHAT_STYLES);

    ChatState::new(mcp_state).build(cx);

    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Element::new(cx).class("chat-pip");
            Label::new(cx, "Synth Control").class("chat-title");
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
