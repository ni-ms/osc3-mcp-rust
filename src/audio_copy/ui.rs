//! The "AUDIO COPY" tab: upload a clip, separate it into auditionable stems, and
//! replicate a chosen stem on the synth (locally or via Gemini).
//!
//! Modeled on `ai::chat_ui` — a Vizia `Model` owning `Arc<SineParams>`, a shared
//! `tokio::Runtime`, and a cancel flag, doing heavy work (decode/separate/match)
//! off the UI thread via `cx.spawn` and writing params through `RawParamEvent`s.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vizia_plug::vizia::prelude::*;

use crate::SineParams;
use crate::ai::llm::AiConfig;

use super::decode::Clip;
use super::playback::PreviewPlayer;
use super::separate::{Engine, Stem};
use super::{analyze, gemini, separate};

pub const AUDIO_COPY_STYLES: &str = r#"
    .ac-root {
        padding: 12px;
        gap: 10px;
        background-color: #0A0A0C;
    }
    .ac-header { height: 26px; gap: 8px; alignment: center; }
    .ac-title {
        color: #F8FAFC;
        font-size: 12px;
        font-weight: 700;
        width: 1s;
    }
    .ac-btn {
        height: 26px;
        background-color: #1C1C22;
        border: 1px solid #2D2D34;
        corner-radius: 4px;
        color: #CBD5E1;
        font-size: 11px;
        padding-left: 12px;
        padding-right: 12px;
        alignment: center;
    }
    .ac-btn:hover { border-color: #6366F1; }
    .ac-btn-primary {
        height: 26px;
        background-color: #6366F1;
        corner-radius: 4px;
        color: #F8FAFC;
        font-size: 11px;
        padding-left: 12px;
        padding-right: 12px;
        alignment: center;
    }
    .ac-btn-primary:hover { background-color: #818CF8; }
    .ac-engine {
        height: 26px;
        background-color: #1C1C22;
        border: 1px solid #2D2D34;
        corner-radius: 4px;
        color: #94A3B8;
        font-size: 10px;
        padding-left: 10px;
        padding-right: 10px;
        alignment: center;
    }
    .ac-engine.selected { background-color: #6366F1; color: #F8FAFC; }
    .ac-row { gap: 6px; height: 26px; alignment: center; }
    .ac-caption { color: #64748B; font-size: 10px; font-weight: 700; }
    .ac-info { color: #94A3B8; font-size: 10px; height: 14px; }
    .ac-status { color: #A5B4FC; font-size: 10px; height: 14px; }
    .ac-list {
        height: 1s;
        background-color: #121216;
        border: 1px solid #2D2D34;
        corner-radius: 6px;
        padding: 6px;
    }
    .ac-stemrow {
        gap: 6px;
        height: 30px;
        alignment: center;
        padding-left: 4px;
        padding-right: 4px;
    }
    .ac-stemlabel { color: #E5E7EB; font-size: 11px; width: 1s; }
    .ac-smallbtn {
        height: 22px;
        background-color: #1C1C22;
        border: 1px solid #2D2D34;
        corner-radius: 4px;
        color: #CBD5E1;
        font-size: 10px;
        padding-left: 8px;
        padding-right: 8px;
        alignment: center;
    }
    .ac-smallbtn:hover { border-color: #6366F1; }
    .ac-smallbtn-primary {
        height: 22px;
        background-color: #6366F1;
        corner-radius: 4px;
        color: #F8FAFC;
        font-size: 10px;
        padding-left: 8px;
        padding-right: 8px;
        alignment: center;
    }
    .ac-smallbtn-primary:hover { background-color: #818CF8; }
"#;

/// A list-friendly view of a stem (the heavy samples live in `stems_full`).
#[derive(Clone, Data)]
pub struct StemView {
    pub label: String,
    pub index: usize,
}

pub enum AudioCopyEvent {
    /// Open the native file dialog and decode the chosen clip.
    PickFile,
    /// A clip finished decoding.
    ClipLoaded(Clip),
    /// Toggle the separation engine (false = DSP, true = ML).
    SetEngineMl(bool),
    /// Run separation on the loaded clip.
    Separate,
    /// Separation finished (stems + optional user-facing note).
    StemsReady(Vec<Stem>, Option<String>),
    /// Separation failed.
    SeparateFailed(String),
    /// Audition the stem at this index.
    Play(usize),
    /// Stop any preview playback.
    StopPlayback,
    /// Replicate the stem locally (synchronous FFT analysis).
    MatchLocal(usize),
    /// Replicate the stem via Gemini (async).
    MatchGemini(usize),
    /// Gemini match finished (summary or error).
    MatchDone(String),
    /// Cancel the in-flight Gemini request.
    CancelTask,
    /// Update the status line.
    Status(String),
}

#[derive(Lens)]
pub struct AudioCopyState {
    params: Arc<SineParams>,
    preview: Arc<PreviewPlayer>,
    runtime: Option<Arc<tokio::runtime::Runtime>>,
    cancel: Arc<AtomicBool>,

    status: String,
    clip_info: String,
    use_ml: bool,
    busy: bool,
    stems: Vec<StemView>,

    /// Decoded clip (not lensed — heavy, used by the Separate handler).
    clip: Option<Clip>,
    /// Full stems with samples (not lensed — used by Play/Match handlers).
    stems_full: Vec<Stem>,
}

impl Model for AudioCopyState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|ev: &AudioCopyEvent, _meta| match ev {
            AudioCopyEvent::PickFile => {
                self.status = "Choose an audio file…".to_string();
                cx.spawn(move |proxy| {
                    let picked = rfd::FileDialog::new()
                        .add_filter(
                            "Audio",
                            &["wav", "mp3", "flac", "ogg", "oga", "aif", "aiff", "m4a", "aac"],
                        )
                        .pick_file();
                    match picked {
                        Some(path) => match super::decode::load_clip(&path) {
                            Ok(clip) => {
                                let _ = proxy.emit(AudioCopyEvent::ClipLoaded(clip));
                            }
                            Err(e) => {
                                let _ = proxy
                                    .emit(AudioCopyEvent::Status(format!("Couldn't load: {e}")));
                            }
                        },
                        None => {
                            let _ = proxy.emit(AudioCopyEvent::Status("Upload cancelled.".into()));
                        }
                    }
                });
            }

            AudioCopyEvent::ClipLoaded(clip) => {
                self.clip_info = format!(
                    "{} · {:.1}s · {} Hz",
                    clip.name, clip.duration_secs, clip.sample_rate
                );
                self.clip = Some(clip.clone());
                self.stems.clear();
                self.stems_full.clear();
                self.status = "Loaded. Click Separate to isolate the elements.".to_string();
            }

            AudioCopyEvent::SetEngineMl(b) => self.use_ml = *b,

            AudioCopyEvent::Separate => {
                let Some(clip) = self.clip.clone() else {
                    self.status = "Upload a clip first.".to_string();
                    return;
                };
                let engine = if self.use_ml { Engine::Ml } else { Engine::Dsp };
                self.busy = true;
                self.status = "Separating…".to_string();
                cx.spawn(move |proxy| match separate::separate(engine, &clip) {
                    Ok((stems, note)) => {
                        let _ = proxy.emit(AudioCopyEvent::StemsReady(stems, note));
                    }
                    Err(e) => {
                        let _ = proxy.emit(AudioCopyEvent::SeparateFailed(e));
                    }
                });
            }

            AudioCopyEvent::StemsReady(stems, note) => {
                self.stems = stems
                    .iter()
                    .enumerate()
                    .map(|(i, s)| StemView {
                        label: s.label.clone(),
                        index: i,
                    })
                    .collect();
                self.stems_full = stems.clone();
                self.busy = false;
                self.status = match note {
                    Some(n) => format!("Found {} stems. {n}", stems.len()),
                    None => format!("Found {} stems. Play one, then Match it.", stems.len()),
                };
            }

            AudioCopyEvent::SeparateFailed(e) => {
                self.busy = false;
                self.status = format!("Separation failed: {e}");
            }

            AudioCopyEvent::Play(i) => {
                if let Some(stem) = self.stems_full.get(*i) {
                    self.preview.set_buffer(stem.samples.clone(), stem.sample_rate);
                    self.preview.play();
                    self.status = format!("▶ {}", stem.label);
                }
            }

            AudioCopyEvent::StopPlayback => {
                self.preview.stop();
            }

            AudioCopyEvent::MatchLocal(i) => {
                let Some(stem) = self.stems_full.get(*i) else {
                    return;
                };
                let preset = analyze::match_local(stem, &self.params);
                let label = stem.label.clone();
                {
                    let mut emit = |evt| {
                        cx.emit(evt);
                    };
                    preset.apply(&self.params, &mut emit);
                }
                self.status = format!("Matched \"{label}\" locally — see the other tabs.");
            }

            AudioCopyEvent::MatchGemini(i) => {
                let Some(stem) = self.stems_full.get(*i).cloned() else {
                    return;
                };
                let Some(rt) = self.runtime.clone() else {
                    self.status = "Async runtime unavailable.".to_string();
                    return;
                };
                let cfg = AiConfig::load();
                if cfg.api_key.trim().is_empty() {
                    self.status =
                        "Set your Gemini API key in the AI ASSIST tab's settings (⚙) first."
                            .to_string();
                    return;
                }
                self.busy = true;
                self.status = format!("Asking Gemini to match \"{}\"…", stem.label);
                self.cancel.store(false, Ordering::Relaxed);

                let params = self.params.clone();
                let cancel = self.cancel.clone();
                cx.spawn(move |proxy| {
                    rt.block_on(gemini::match_gemini(proxy, &params, &cfg, &stem, cancel));
                });
            }

            AudioCopyEvent::MatchDone(msg) => {
                self.busy = false;
                self.status = msg.clone();
            }

            AudioCopyEvent::CancelTask => {
                self.cancel.store(true, Ordering::Relaxed);
                self.busy = false;
                self.preview.stop();
                self.status = "Stopped.".to_string();
            }

            AudioCopyEvent::Status(s) => self.status = s.clone(),
        });
    }
}

/// Build the Audio Copy panel. `preview` is the shared RT-safe player read by the
/// audio thread.
pub fn audio_copy_panel(cx: &mut Context, params: Arc<SineParams>, preview: Arc<PreviewPlayer>) {
    AudioCopyState {
        params,
        preview,
        runtime: tokio::runtime::Runtime::new().ok().map(Arc::new),
        cancel: Arc::new(AtomicBool::new(false)),
        status: "Upload an audio clip to begin.".to_string(),
        clip_info: String::new(),
        use_ml: false,
        busy: false,
        stems: Vec::new(),
        clip: None,
        stems_full: Vec::new(),
    }
    .build(cx);

    VStack::new(cx, |cx| {
        // Header: title, upload, stop-playback.
        HStack::new(cx, |cx| {
            Label::new(cx, "AUDIO COPY").class("ac-title");
            Button::new(cx, |cx| Label::new(cx, "Upload clip"))
                .on_press(|cx| cx.emit(AudioCopyEvent::PickFile))
                .class("ac-btn-primary");
            Button::new(cx, |cx| Label::new(cx, "■ Stop"))
                .on_press(|cx| cx.emit(AudioCopyEvent::StopPlayback))
                .class("ac-btn");
        })
        .class("ac-header");

        Label::new(cx, AudioCopyState::clip_info).class("ac-info");

        // Engine toggle + Separate.
        HStack::new(cx, |cx| {
            Label::new(cx, "ENGINE").class("ac-caption");
            Button::new(cx, |cx| Label::new(cx, "DSP"))
                .on_press(|cx| cx.emit(AudioCopyEvent::SetEngineMl(false)))
                .class("ac-engine")
                .toggle_class("selected", AudioCopyState::use_ml.map(|m| !*m));
            Button::new(cx, |cx| Label::new(cx, "ML"))
                .on_press(|cx| cx.emit(AudioCopyEvent::SetEngineMl(true)))
                .class("ac-engine")
                .toggle_class("selected", AudioCopyState::use_ml.map(|m| *m));

            Element::new(cx).width(Stretch(1.0)).height(Pixels(0.0));

            Button::new(cx, |cx| Label::new(cx, "Separate"))
                .on_press(|cx| cx.emit(AudioCopyEvent::Separate))
                .class("ac-btn-primary");

            // While busy, offer a cancel.
            Binding::new(cx, AudioCopyState::busy, |cx, busy| {
                if busy.get(cx) {
                    Button::new(cx, |cx| Label::new(cx, "Cancel"))
                        .on_press(|cx| cx.emit(AudioCopyEvent::CancelTask))
                        .class("ac-btn");
                }
            });
        })
        .class("ac-row");

        Label::new(cx, AudioCopyState::status).class("ac-status");

        // Stems list.
        ScrollView::new(cx, |cx| {
            List::new(cx, AudioCopyState::stems, |cx, index, item| {
                HStack::new(cx, move |cx| {
                    Label::new(cx, item.map(|s| s.label.clone())).class("ac-stemlabel");
                    Button::new(cx, |cx| Label::new(cx, "▶"))
                        .on_press(move |cx| cx.emit(AudioCopyEvent::Play(index)))
                        .class("ac-smallbtn");
                    Button::new(cx, |cx| Label::new(cx, "Match (Local)"))
                        .on_press(move |cx| cx.emit(AudioCopyEvent::MatchLocal(index)))
                        .class("ac-smallbtn-primary");
                    Button::new(cx, |cx| Label::new(cx, "Gemini"))
                        .on_press(move |cx| cx.emit(AudioCopyEvent::MatchGemini(index)))
                        .class("ac-smallbtn");
                })
                .class("ac-stemrow");
            });
        })
        .class("ac-list");
    })
    .class("ac-root");
}
