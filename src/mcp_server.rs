// use rmcp::{Error, ServerHandler, ServiceExt, model::*, schemars, tool};
// use serde::{Deserialize, Serialize};
// use std::sync::Arc;
// use crate::{SineParams, Waveform, FilterMode};
//
// #[derive(Clone)]
// pub struct VSTMCPServer {
//     pub params: Arc<SineParams>,
// }
//
// #[derive(Serialize, Deserialize, schemars::JsonSchema)]
// struct UpdateOscillatorParams {
//     #[schemars(description = "Oscillator number (1, 2, or 3)")]
//     oscillator: u8,
//     #[schemars(description = "Waveform type: sine, square, triangle, sawtooth")]
//     waveform: Option<String>,
//     #[schemars(description = "Frequency in Hz")]
//     frequency: Option<f32>,
//     #[schemars(description = "Detune in cents")]
//     detune: Option<f32>,
//     #[schemars(description = "Phase offset (0.0 to 1.0)")]
//     phase: Option<f32>,
//     #[schemars(description = "Gain in dB")]
//     gain: Option<f32>,
//     #[schemars(description = "Octave offset (-4 to 4)")]
//     octave: Option<i32>,
//     #[schemars(description = "Number of unison voices (1 to 8)")]
//     unison_voices: Option<u8>,
//     #[schemars(description = "Unison detune in cents")]
//     unison_detune: Option<f32>,
//     #[schemars(description = "Unison blend (0.0 to 1.0)")]
//     unison_blend: Option<f32>,
//     #[schemars(description = "Unison volume (0.0 to 1.0)")]
//     unison_volume: Option<f32>,
// }
//
// #[derive(Serialize, Deserialize, schemars::JsonSchema)]
// struct UpdateFilterParams {
//     #[schemars(description = "Filter mode: lowpass, highpass, bandpass, notch")]
//     mode: Option<String>,
//     #[schemars(description = "Cutoff frequency in Hz")]
//     cutoff: Option<f32>,
//     #[schemars(description = "Resonance (0.0 to 1.0)")]
//     resonance: Option<f32>,
//     #[schemars(description = "Drive (1.0 to 5.0)")]
//     drive: Option<f32>,
// }
//
// #[derive(Serialize, Deserialize, schemars::JsonSchema)]
// struct UpdateEnvelopeParams {
//     #[schemars(description = "Attack time in seconds")]
//     attack: Option<f32>,
//     #[schemars(description = "Decay time in seconds")]
//     decay: Option<f32>,
//     #[schemars(description = "Sustain level (0.0 to 1.0)")]
//     sustain: Option<f32>,
//     #[schemars(description = "Release time in seconds")]
//     release: Option<f32>,
// }
//
//
// impl ServerHandler for VSTMCPServer {
//     fn get_info(&self) -> ServerInfo {
//         ServerInfo {
//             protocol_version: ProtocolVersion::V_2024_11_05,
//             capabilities: ServerCapabilities::builder()
//                 .enable_tools()
//                 .build(),
//             server_info: Implementation::from_build_env(),
//             instructions: Some("VST Triple Oscillator Synthesizer Parameter Control - Manage all synthesizer parameters in real-time".to_string()),
//         }
//     }
// }
//
// #[tool(tool_box)]
// impl VSTMCPServer {
//     #[tool(description = "Get current state of all parameters")]
//     async fn get_all_parameters(&self) -> Result<CallToolResult, Error> {
//         let params_json = serde_json::json!({
//             "oscillator1": {
//                 "waveform": format!("{:?}", self.params.waveform1.value()).to_lowercase(),
//                 "frequency": self.params.frequency1.value(),
//                 "detune": self.params.detune1.value(),
//                 "phase": self.params.phase1.value(),
//                 "gain_db": util::gain_to_db(self.params.gain1.value()),
//                 "octave": self.params.octave1.value(),
//                 "unison_voices": self.params.unison_voices1.value(),
//                 "unison_detune": self.params.unison_detune1.value(),
//                 "unison_blend": self.params.unison_blend1.value(),
//                 "unison_volume": self.params.unison_volume1.value(),
//             },
//             "oscillator2": {
//                 "waveform": format!("{:?}", self.params.waveform2.value()).to_lowercase(),
//                 "frequency": self.params.frequency2.value(),
//                 "detune": self.params.detune2.value(),
//                 "phase": self.params.phase2.value(),
//                 "gain_db": util::gain_to_db(self.params.gain2.value()),
//                 "octave": self.params.octave2.value(),
//                 "unison_voices": self.params.unison_voices2.value(),
//                 "unison_detune": self.params.unison_detune2.value(),
//                 "unison_blend": self.params.unison_blend2.value(),
//                 "unison_volume": self.params.unison_volume2.value(),
//             },
//             "oscillator3": {
//                 "waveform": format!("{:?}", self.params.waveform3.value()).to_lowercase(),
//                 "frequency": self.params.frequency3.value(),
//                 "detune": self.params.detune3.value(),
//                 "phase": self.params.phase3.value(),
//                 "gain_db": util::gain_to_db(self.params.gain3.value()),
//                 "octave": self.params.octave3.value(),
//                 "unison_voices": self.params.unison_voices3.value(),
//                 "unison_detune": self.params.unison_detune3.value(),
//                 "unison_blend": self.params.unison_blend3.value(),
//                 "unison_volume": self.params.unison_volume3.value(),
//             },
//             "filter": {
//                 "mode": format!("{:?}", self.params.filter_mode.value()).to_lowercase(),
//                 "cutoff": self.params.filter_cutoff.value(),
//                 "resonance": self.params.filter_resonance.value(),
//                 "drive": self.params.filter_drive.value(),
//             },
//             "envelope": {
//                 "attack": self.params.attack.value(),
//                 "decay": self.params.decay.value(),
//                 "sustain": self.params.sustain.value(),
//                 "release": self.params.release.value(),
//             }
//         });
//
//         Ok(CallToolResult::success(vec![Content::text(
//             serde_json::to_string_pretty(&params_json).unwrap()
//         )]))
//     }
//
//     #[tool(description = "Update oscillator parameters")]
//     async fn update_oscillator(
//         &self,
//         #[tool(aggr)] params: UpdateOscillatorParams,
//     ) -> Result<CallToolResult, Error> {
//         if params.oscillator < 1 || params.oscillator > 3 {
//             return Err(Error::invalid_request("Oscillator must be 1, 2, or 3"));
//         }
//
//         let mut updates = Vec::new();
//
//         match params.oscillator {
//             1 => {
//                 if let Some(waveform) = &params.waveform {
//                     if let Ok(wf) = self.parse_waveform(waveform) {
//                         self.params.waveform1.set_value(wf);
//                         updates.push(format!("waveform1: {}", waveform));
//                     }
//                 }
//                 if let Some(freq) = params.frequency {
//                     self.params.frequency1.set_value(freq.clamp(20.0, 20000.0));
//                     updates.push(format!("frequency1: {}", freq));
//                 }
//                 if let Some(detune) = params.detune {
//                     self.params.detune1.set_value(detune.clamp(-100.0, 100.0));
//                     updates.push(format!("detune1: {}", detune));
//                 }
//                 if let Some(phase) = params.phase {
//                     self.params.phase1.set_value(phase.clamp(0.0, 1.0));
//                     updates.push(format!("phase1: {}", phase));
//                 }
//                 if let Some(gain_db) = params.gain {
//                     let gain = util::db_to_gain(gain_db.clamp(-36.0, 0.0));
//                     self.params.gain1.set_value(gain);
//                     updates.push(format!("gain1: {} dB", gain_db));
//                 }
//                 if let Some(octave) = params.octave {
//                     self.params.octave1.set_value(octave.clamp(-4, 4));
//                     updates.push(format!("octave1: {}", octave));
//                 }
//                 if let Some(voices) = params.unison_voices {
//                     self.params.unison_voices1.set_value(voices.clamp(1, 8) as i32);
//                     updates.push(format!("unison_voices1: {}", voices));
//                 }
//                 if let Some(unison_detune) = params.unison_detune {
//                     self.params.unison_detune1.set_value(unison_detune.clamp(0.0, 50.0));
//                     updates.push(format!("unison_detune1: {}", unison_detune));
//                 }
//                 if let Some(blend) = params.unison_blend {
//                     self.params.unison_blend1.set_value(blend.clamp(0.0, 1.0));
//                     updates.push(format!("unison_blend1: {}", blend));
//                 }
//                 if let Some(volume) = params.unison_volume {
//                     self.params.unison_volume1.set_value(volume.clamp(0.0, 1.0));
//                     updates.push(format!("unison_volume1: {}", volume));
//                 }
//             }
//             2 => {
//                 // Similar implementation for oscillator 2
//                 if let Some(waveform) = &params.waveform {
//                     if let Ok(wf) = self.parse_waveform(waveform) {
//                         self.params.waveform2.set_value(wf);
//                         updates.push(format!("waveform2: {}", waveform));
//                     }
//                 }
//                 if let Some(freq) = params.frequency {
//                     self.params.frequency2.set_value(freq.clamp(20.0, 20000.0));
//                     updates.push(format!("frequency2: {}", freq));
//                 }
//
//             }
//             3 => {
//
//                 if let Some(waveform) = &params.waveform {
//                     if let Ok(wf) = self.parse_waveform(waveform) {
//                         self.params.waveform3.set_value(wf);
//                         updates.push(format!("waveform3: {}", waveform));
//                     }
//                 }
//                 if let Some(freq) = params.frequency {
//                     self.params.frequency3.set_value(freq.clamp(20.0, 20000.0));
//                     updates.push(format!("frequency3: {}", freq));
//                 }
//
//             }
//             _ => unreachable!(),
//         }
//
//         Ok(CallToolResult::success(vec![Content::text(format!(
//             "Updated oscillator {} parameters: {}",
//             params.oscillator,
//             updates.join(", ")
//         ))]))
//     }
//
//     #[tool(description = "Update filter parameters")]
//     async fn update_filter(
//         &self,
//         #[tool(aggr)] params: UpdateFilterParams,
//     ) -> Result<CallToolResult, Error> {
//         let mut updates = Vec::new();
//
//         if let Some(mode) = &params.mode {
//             if let Ok(filter_mode) = self.parse_filter_mode(mode) {
//                 self.params.filter_mode.set_value(filter_mode);
//                 updates.push(format!("mode: {}", mode));
//             }
//         }
//
//         if let Some(cutoff) = params.cutoff {
//             self.params.filter_cutoff.set_value(cutoff.clamp(20.0, 20000.0));
//             updates.push(format!("cutoff: {} Hz", cutoff));
//         }
//
//         if let Some(resonance) = params.resonance {
//             self.params.filter_resonance.set_value(resonance.clamp(0.0, 1.0));
//             updates.push(format!("resonance: {}", resonance));
//         }
//
//         if let Some(drive) = params.drive {
//             self.params.filter_drive.set_value(drive.clamp(1.0, 5.0));
//             updates.push(format!("drive: {}", drive));
//         }
//
//         Ok(CallToolResult::success(vec![Content::text(format!(
//             "Updated filter parameters: {}",
//             updates.join(", ")
//         ))]))
//     }
//
//     #[tool(description = "Update ADSR envelope parameters")]
//     async fn update_envelope(
//         &self,
//         #[tool(aggr)] params: UpdateEnvelopeParams,
//     ) -> Result<CallToolResult, Error> {
//         let mut updates = Vec::new();
//
//         if let Some(attack) = params.attack {
//             self.params.attack.set_value(attack.clamp(0.001, 5.0));
//             updates.push(format!("attack: {} s", attack));
//         }
//
//         if let Some(decay) = params.decay {
//             self.params.decay.set_value(decay.clamp(0.001, 5.0));
//             updates.push(format!("decay: {} s", decay));
//         }
//
//         if let Some(sustain) = params.sustain {
//             self.params.sustain.set_value(sustain.clamp(0.0, 1.0));
//             updates.push(format!("sustain: {}", sustain));
//         }
//
//         if let Some(release) = params.release {
//             self.params.release.set_value(release.clamp(0.001, 10.0));
//             updates.push(format!("release: {} s", release));
//         }
//
//         Ok(CallToolResult::success(vec![Content::text(format!(
//             "Updated envelope parameters: {}",
//             updates.join(", ")
//         ))]))
//     }
//
//     fn parse_waveform(&self, waveform: &str) -> Result<Waveform, Error> {
//         match waveform.to_lowercase().as_str() {
//             "sine" => Ok(Waveform::Sine),
//             "square" => Ok(Waveform::Square),
//             "triangle" => Ok(Waveform::Triangle),
//             "sawtooth" => Ok(Waveform::Sawtooth),
//             _ => Err(Error::invalid_request("Invalid waveform. Use: sine, square, triangle, sawtooth")),
//         }
//     }
//
//     fn parse_filter_mode(&self, mode: &str) -> Result<FilterMode, Error> {
//         match mode.to_lowercase().as_str() {
//             "lowpass" => Ok(FilterMode::LowPass),
//             "highpass" => Ok(FilterMode::HighPass),
//             "bandpass" => Ok(FilterMode::BandPass),
//             "notch" => Ok(FilterMode::Notch),
//             _ => Err(Error::invalid_request("Invalid filter mode. Use: lowpass, highpass, bandpass, notch")),
//         }
//     }
// }
