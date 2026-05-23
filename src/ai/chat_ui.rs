// use crate::mcp_server::{AiModel, PluginState};
// use base64::{engine::general_purpose, Engine as _};
// use reqwest::StatusCode;
// use std::sync::Arc;
// use tokio::sync::RwLock;
// use vizia_plug::vizia::prelude::*;
// use vizia_plug::vizia::vg::font_style::Weight;
//
// const CHAT_STYLES: &str = r#"
// .chat-root { background-color: #18181E; border: 1px solid #334155; border-radius: 6px; padding: 8px; height: 400px; }
// .chat-header { height: 24px; flex-shrink: 0; align: center; justify-content: space-between; }
// .chat-transcript { background-color: #121216; border: 1px solid #334155; border-radius: 4px; padding: 8px; flex-grow: 1; }
// .chat-text { color: #E5E7EB; font-size: 11px; line-height: 1.35; white-space: pre-wrap; }
// .chat-input { background-color: #0F1115; border: 1px solid #334155; border-radius: 4px; color: #F8FAFC; font-size: 11px; height: 26px; padding-left: 6px; }
// .chat-send { height: 26px; background-color: #202028; border: 1px solid #334155; border-radius: 4px; color: #F8FAFC; font-size: 11px; padding: 0 8px; }
// .chat-send:hover { background-color: #1E293B; }
//
// /* Settings Overlay */
// .settings-overlay { position-type: self-directed; width: 100%; height: 100%; background-color: #18181ed9; padding: 12px; }
// .settings-label { color: #94A3B8; font-size: 10px; font-weight: 700; }
// .model-btn { background-color: #2D2D39; padding: 4px; font-size: 10px; border-radius: 3px; }
// .model-btn.selected { background-color: #6366F1; color: white; }
//
// /* Attachment Preview */
// .attachment-preview { background-color: #2D2D39; border-radius: 4px; padding: 4px; margin-bottom: 4px; align-items: center; }
// "#;
//
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Data)]
// pub enum Role {
//     User,
//     Assistant,
// }
//
// #[derive(Clone, Lens, Data)]
// pub struct ChatMessage {
//     pub role: Role,
//     pub text: String,
// }
//
// #[derive(Lens)]
// pub struct ChatState {
//     pub messages: Vec<ChatMessage>,
//     pub input: String,
//     pub sending: bool,
//     pub is_settings_open: bool,
//     pub pending_audio: Option<Vec<u8>>,
//     pub pending_filename: Option<String>,
// }
//
// pub enum ChatEvent {
//     EditInput(String),
//     Send(Arc<RwLock<PluginState>>),
//     Receive(String),
//     ToggleSettings,
//     PickAudio,
//     ClearAudio,
//     SetModel(AiModel, Arc<RwLock<PluginState>>),
//     SetApiKey(String, Arc<RwLock<PluginState>>),
// }
//
// impl Model for ChatState {
//     fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
//         event.map(|ev: &ChatEvent, _| match ev {
//             ChatEvent::EditInput(s) => self.input = s.clone(),
//             ChatEvent::ToggleSettings => self.is_settings_open = !self.is_settings_open,
//             ChatEvent::PickAudio => {
//                 if let Some(path) = rfd::FileDialog::new()
//                     .add_filter("Audio", &["wav", "mp3"])
//                     .pick_file()
//                 {
//                     if let Ok(bytes) = std::fs::read(&path) {
//                         self.pending_filename =
//                             Some(path.file_name().unwrap().to_string_lossy().to_string());
//                         self.pending_audio = Some(bytes);
//                     }
//                 }
//             }
//             ChatEvent::ClearAudio => {
//                 self.pending_audio = None;
//                 self.pending_filename = None;
//             }
//             ChatEvent::SetModel(m, state) => {
//                 let state = state.clone();
//                 let m = *m;
//                 cx.needs_redraw();
//                 std::thread::spawn(move || {
//                     let rt = tokio::runtime::Runtime::new().unwrap();
//                     rt.block_on(async move {
//                         state.write().await.model = m;
//                     });
//                 });
//             }
//             ChatEvent::SetApiKey(k, state) => {
//                 let state = state.clone();
//                 let k = k.clone();
//                 cx.spawn(|cx| {
//                     let state = state.clone();
//                     let k = k.clone();
//                     cx.spawn(async move { state.write().await.api_key = k; });
//                 });
//             }
//             ChatEvent::Send(state_arc) => {
//                 let text = std::mem::take(&mut self.input);
//                 if text.trim().is_empty() && self.pending_audio.is_none() {
//                     return;
//                 }
//
//                 self.messages.push(ChatMessage {
//                     role: Role::User,
//                     text: text.clone(),
//                 });
//                 self.sending = true;
//
//                 let state_arc = state_arc.clone();
//                 let audio_bytes = self.pending_audio.take();
//                 self.pending_filename = None;
//                 let text_clone = text.clone();
//
//                 cx.spawn(|cx| {
//                     let state_arc = state_arc.clone();
//                     let audio_bytes = audio_bytes.clone();
//                     let text_clone = text_clone.clone();
//                     cx.spawn(async move {
//                         let (key, model, temp) = {
//                             let s = state_arc.read().await;
//                             (s.api_key.clone(), s.model.api_name().to_string(), s.temperature)
//                         };
//
//                         if key.is_empty() {
//                             cx.emit(ChatEvent::Receive("Please set an API Key in settings (⚙)".into()));
//                             return;
//                         }
//
//                         let client = reqwest::Client::new();
//                         let url = format!(
//                             "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
//                             model, key
//                         );
//
//                         let mut parts = vec![serde_json::json!({ "text": text_clone })];
//                         if let Some(audio) = audio_bytes {
//                             parts.push(serde_json::json!({
//                     "inline_data": {
//                         "mime_type": "audio/wav",
//                         "data": general_purpose::STANDARD.encode(audio)
//                     }
//                 }));
//                         }
//
//                         let body = serde_json::json!({
//                 "contents": [{ "parts": parts }],
//                 "tools": PluginState::get_tools_as_gemini_schema(),
//                 "generationConfig": { "temperature": temp }
//             });
//
//                         match client.post(url).json(&body).send().await {
//                             Ok(resp) => {
//                                 if resp.status() == StatusCode::OK {
//                                     if let Ok(json) = resp.json::<serde_json::Value>().await {
//                                         // Tool calls
//                                         if let Some(candidates) = json.get("candidates") {
//                                             if let Some(candidate) = candidates.get(0) {
//                                                 if let Some(content) = candidate.get("content") {
//                                                     if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
//                                                         for part in parts {
//                                                             if let Some(call) = part.get("functionCall") {
//                                                                 let name = call["name"].as_str().unwrap_or("");
//                                                                 let args = call["args"].clone();
//                                                                 let _ = PluginState::execute_tool(state_arc.clone(), name, args).await;
//                                                             }
//                                                         }
//                                                     }
//                                                 }
//                                             }
//                                         }
//                                         // Reply text
//                                         let reply = json
//                                             .get("candidates")
//                                             .and_then(|c| c.get(0))
//                                             .and_then(|cand| cand.get("content"))
//                                             .and_then(|content| content.get("parts"))
//                                             .and_then(|parts| parts.get(0))
//                                             .and_then(|part| part.get("text"))
//                                             .and_then(|t| t.as_str())
//                                             .unwrap_or("Sound updated!")
//                                             .to_string();
//                                         cx.emit(ChatEvent::Receive(reply));
//                                     } else {
//                                         cx.emit(ChatEvent::Receive("Parse error".into()));
//                                     }
//                                 } else {
//                                     cx.emit(ChatEvent::Receive(format!("API error: {}", resp.status()))).ok();
//                                 }
//                             }
//                             Err(_) => {
//                                 cx.emit(ChatEvent::Receive("Network error".into()));
//                             }
//                         }
//                     });
//                 });
//             }
//
//             ChatEvent::Receive(reply) => {
//                 self.sending = false;
//                 self.messages.push(ChatMessage {
//                     role: Role::Assistant,
//                     text: reply.clone(),
//                 });
//             }
//         });
//     }
// }
//
// pub fn chat_panel(cx: &mut Context, mcp_state: Arc<RwLock<PluginState>>) -> Handle<'_, impl View> {
//     cx.add_stylesheet(CHAT_STYLES);
//     ChatState {
//         messages: vec![],
//         input: "".into(),
//         sending: false,
//         is_settings_open: false,
//         pending_audio: None,
//         pending_filename: None,
//     }
//     .build(cx);
//
//     VStack::new(cx, |cx| {
//         // Header
//         HStack::new(cx, |cx| {
//             HStack::new(cx, |cx| {
//                 Element::new(cx).class("chat-pip");
//                 Label::new(cx, "AI SYNTH AGENT").class("chat-title");
//             })
//             .horizontal_gap(Pixels(6.0));
//
//             Button::new(cx, |cx| Label::new(cx, "⚙"))
//                 .on_press(|cx| cx.emit(ChatEvent::ToggleSettings))
//                 .class("chat-send");
//         })
//         .class("chat-header");
//
//         // Transcript
//         ScrollView::new(cx, |cx| {
//             List::new(cx, ChatState::messages, |cx, _, item| {
//                 VStack::new(cx, |cx| {
//                     Label::new(
//                         cx,
//                         item.map(|m| if m.role == Role::User { "YOU" } else { "AI" }),
//                     )
//                     .font_weight(*Weight::BOLD)
//                     .font_size(9.0);
//
//                     Label::new(cx, item.map(|m| m.text.clone()))
//                         .class("chat-text")
//                         .width(Stretch(1.0));
//                 })
//                 .vertical_gap(Pixels(4.0));
//             });
//         })
//         .class("chat-transcript");
//
//         // Audio Attachment Preview
//         Binding::new(cx, ChatState::pending_filename, |cx, file| {
//             if let Some(name) = file.get(cx) {
//                 HStack::new(cx, |cx| {
//                     Label::new(cx, format!("📎 {}", name)).class("chat-text");
//                     Button::new(cx, |cx| Label::new(cx, "✕"))
//                         .on_press(|cx| cx.emit(ChatEvent::ClearAudio));
//                 })
//                 .class("attachment-preview")
//                 .horizontal_gap(Pixels(6.0));
//             }
//         });
//
//         // Input Area
//         HStack::new(cx, |cx| {
//             Button::new(cx, |cx| Label::new(cx, "📁"))
//                 .on_press(|cx| cx.emit(ChatEvent::PickAudio))
//                 .class("chat-send");
//
//             let state_for_input = mcp_state.clone();
//             Textbox::new(cx, ChatState::input)
//                 .class("chat-input")
//                 .width(Stretch(1.0))
//                 .on_edit(|cx, text| cx.emit(ChatEvent::EditInput(text)))
//                 .on_submit(move |cx, _, _| cx.emit(ChatEvent::Send(state_for_input.clone())));
//
//             let state_for_btn = mcp_state.clone();
//             Button::new(cx, |cx| Label::new(cx, "Send"))
//                 .on_press(move |cx| cx.emit(ChatEvent::Send(state_for_btn.clone())))
//                 .class("chat-send");
//         })
//         .horizontal_gap(Pixels(6.0));
//
//         // Settings Overlay
//         Binding::new(cx, ChatState::is_settings_open, move |cx, open| {
//             if open.get(cx) {
//                 VStack::new(cx, move |cx| {
//                     Label::new(cx, "AI SETTINGS").class("settings-label");
//
//                     // API KEY
//                     let s1 = mcp_state.clone();
//                     Textbox::new(cx, String::new())
//                         .placeholder("Gemini API Key")
//                         .on_edit(move |cx, k| cx.emit(ChatEvent::SetApiKey(k, s1.clone())))
//                         .class("chat-input");
//
//                     // MODEL SELECTION
//                     Label::new(cx, "MODEL").class("settings-label");
//                     HStack::new(cx, move |cx| {
//                         let s = mcp_state.clone();
//                         Button::new(cx, |cx| Label::new(cx, "1.5 Flash"))
//                             .on_press(move |cx| {
//                                 cx.emit(ChatEvent::SetModel(AiModel::Gemini15Flash, s.clone()))
//                             })
//                             .class("model-btn");
//
//                         let s = mcp_state.clone();
//                         Button::new(cx, |cx| Label::new(cx, "1.5 Pro"))
//                             .on_press(move |cx| {
//                                 cx.emit(ChatEvent::SetModel(AiModel::Gemini15Pro, s.clone()))
//                             })
//                             .class("model-btn");
//                     })
//                     .horizontal_gap(Pixels(4.0));
//
//                     Button::new(cx, |cx| Label::new(cx, "DONE"))
//                         .on_press(|cx| cx.emit(ChatEvent::ToggleSettings))
//                         .class("chat-send")
//                         .top(Pixels(10.0));
//                 })
//                 .class("settings-overlay")
//                 .vertical_gap(Pixels(8.0));
//             }
//         });
//     })
//     .class("chat-root")
// }
