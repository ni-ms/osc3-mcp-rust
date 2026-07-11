//! Audio Copy: upload a clip, isolate its elements into auditionable stems, and
//! replicate a chosen stem on the synth (locally or via the cloud AI).
//!
//! - [`decode`] — load/decode a file to mono `f32` (symphonia).
//! - [`separate`] — the `Separator` trait + the DSP engine (HPSS + band split)
//!   and an ML stub behind the same trait.
//! - [`analyze`] — local FFT analysis → a [`crate::ai::preset::PresetData`] patch.
//! - [`cloud`] — send a stem as inline audio and let an OpenRouter model design
//!   the patch.
//! - [`playback`] — [`PreviewPlayer`], the RT-safe audition hand-off.
//! - [`ui`] — the "AUDIO COPY" tab (Vizia model + view).

pub mod analyze;
pub mod cloud;
pub mod decode;
pub mod playback;
pub mod separate;
pub mod ui;

pub use playback::PreviewPlayer;
pub use ui::{AUDIO_COPY_STYLES, audio_copy_panel};
