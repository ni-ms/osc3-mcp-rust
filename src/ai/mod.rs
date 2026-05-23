//! AI assistant layer (Gemini via MCP-style tool calls).
//!
//! **This feature is currently inert.** [`chat_ui`] is not wired into the
//! editor, and [`mcp::start_mcp_server`] only spawns an idle thread. The
//! [`mcp::PluginState`] struct holds its own copy of the parameter values and
//! is **not** connected to the live audio parameters — changing it does not
//! affect the sound. Before enabling this, route AI parameter writes through the
//! real [`crate::SineParams`] (e.g. a lock-free command queue drained in
//! `process`, or `nih_plug`'s `BackgroundTask`/`AsyncExecutor`). See
//! `ARCHITECTURE_REVIEW.md`.

pub mod chat_ui;
pub mod mcp;
