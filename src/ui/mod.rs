//! GUI layer: the `vizia` editor and its reusable view components.
//!
//! - [`editor`] assembles the whole window (header, tabs, module cards).
//! - [`knob`], [`tab_switcher`], [`meter`] are self-contained, reusable widgets
//!   that the editor composes. Each owns its own CSS and event handling, so they
//!   can be dropped into any `vizia` tree.
//!
//! [`PeakMeter`] is the lock-free hand-off between the audio thread and the
//! [`Meter`] view; it lives here next to its consumer but is written from
//! `SineSynth::process` (see `lib.rs`).

pub mod editor;
pub mod knob;
pub mod meter;
pub mod tab_switcher;

pub use knob::ParamKnob;
pub use meter::{Meter, PeakMeter};
pub use tab_switcher::{TabDefinition, TabSwitcher};
