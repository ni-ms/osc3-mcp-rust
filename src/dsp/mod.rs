//! Pure DSP primitives. These types contain only `f32` math and depend on the
//! parameter enums (`Waveform`, `FilterMode`) but never on `nih_plug` plumbing.

pub mod envelope;
pub mod filter;
pub mod oscillator;
pub mod voice;

pub use voice::{FrameParams, Voice};