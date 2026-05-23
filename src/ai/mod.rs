//! AI assistant layer: an in-plugin chat panel that drives the synth through
//! the Gemini tool-calling API and reads/writes JSON presets on disk.
//!
//! - [`chat_ui`] — the "AI ASSIST" tab (Vizia model + view).
//! - [`llm`] — Gemini config + the multi-turn tool-calling loop.
//! - [`tools`] — tool schemas + the in-plugin dispatcher.
//! - [`bridge`] — maps tool calls to real `nih_plug` parameter writes.
//! - [`preset`] — parameter snapshot capture/apply + JSON file storage.

pub mod bridge;
pub mod chat_ui;
pub mod llm;
pub mod preset;
pub mod tools;
