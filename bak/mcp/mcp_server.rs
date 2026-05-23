// mcp_server.rs
use crate::{FilterMode, Waveform};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{
    handler::server::router::tool::ToolRouter, model::{ErrorData as McpError, *},
    schemars,
    tool,
    tool_handler, tool_router, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct SynthMcpServer {
    params: Arc<RwLock<PluginState>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PluginState {
    pub osc1_waveform: String,
    pub osc1_frequency: f32,
    pub osc1_detune: f32,
    pub osc1_phase: f32,
    pub osc1_gain: f32,
    pub osc1_octave: i32,
    pub osc1_unison_voices: i32,
    pub osc1_unison_detune: f32,
    pub osc1_unison_blend: f32,
    pub osc1_unison_volume: f32,

    pub osc2_waveform: String,
    pub osc2_frequency: f32,
    pub osc2_detune: f32,
    pub osc2_phase: f32,
    pub osc2_gain: f32,
    pub osc2_octave: i32,
    pub osc2_unison_voices: i32,
    pub osc2_unison_detune: f32,
    pub osc2_unison_blend: f32,
    pub osc2_unison_volume: f32,

    pub osc3_waveform: String,
    pub osc3_frequency: f32,
    pub osc3_detune: f32,
    pub osc3_phase: f32,
    pub osc3_gain: f32,
    pub osc3_octave: i32,
    pub osc3_unison_voices: i32,
    pub osc3_unison_detune: f32,
    pub osc3_unison_blend: f32,
    pub osc3_unison_volume: f32,

    pub filter_mode: String,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_drive: f32,

    pub envelope_attack: f32,
    pub envelope_decay: f32,
    pub envelope_sustain: f32,
    pub envelope_release: f32,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            osc1_waveform: "Sine".to_string(),
            osc1_frequency: 440.0,
            osc1_detune: 0.0,
            osc1_phase: 0.0,
            osc1_gain: 0.5,
            osc1_octave: 0,
            osc1_unison_voices: 1,
            osc1_unison_detune: 0.0,
            osc1_unison_blend: 0.0,
            osc1_unison_volume: 1.0,

            osc2_waveform: "Sawtooth".to_string(),
            osc2_frequency: 880.0,
            osc2_detune: 0.0,
            osc2_phase: 0.0,
            osc2_gain: 0.25,
            osc2_octave: -1,
            osc2_unison_voices: 1,
            osc2_unison_detune: 0.0,
            osc2_unison_blend: 0.0,
            osc2_unison_volume: 1.0,

            osc3_waveform: "Square".to_string(),
            osc3_frequency: 220.0,
            osc3_detune: 0.0,
            osc3_phase: 0.0,
            osc3_gain: 0.125,
            osc3_octave: 1,
            osc3_unison_voices: 1,
            osc3_unison_detune: 0.0,
            osc3_unison_blend: 0.0,
            osc3_unison_volume: 1.0,

            filter_mode: "LowPass".to_string(),
            filter_cutoff: 20000.0,
            filter_resonance: 0.0,
            filter_drive: 1.0,

            envelope_attack: 0.01,
            envelope_decay: 0.5,
            envelope_sustain: 0.7,
            envelope_release: 1.0,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetStateParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetOscillatorParams {
    #[schemars(description = "Oscillator number (1, 2, or 3)")]
    pub oscillator: u8,

    #[schemars(description = "Waveform type: Sine, Square, Triangle, or Sawtooth")]
    pub waveform: Option<String>,

    #[schemars(description = "Frequency in Hz (20-20000)")]
    pub frequency: Option<f32>,

    #[schemars(description = "Detune in cents (-100 to 100)")]
    pub detune: Option<f32>,

    #[schemars(description = "Phase offset (0.0-1.0)")]
    pub phase: Option<f32>,

    #[schemars(description = "Gain level (0.0-1.0)")]
    pub gain: Option<f32>,

    #[schemars(description = "Octave shift (-4 to 4)")]
    pub octave: Option<i32>,

    #[schemars(description = "Number of unison voices (1-8)")]
    pub unison_voices: Option<i32>,

    #[schemars(description = "Unison detune amount in cents (0-50)")]
    pub unison_detune: Option<f32>,

    #[schemars(description = "Unison blend (0.0-1.0)")]
    pub unison_blend: Option<f32>,

    #[schemars(description = "Unison volume (0.0-1.0)")]
    pub unison_volume: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetFilterParams {
    #[schemars(description = "Filter mode: LowPass, HighPass, BandPass, or Notch")]
    pub mode: Option<String>,

    #[schemars(description = "Cutoff frequency in Hz (20-20000)")]
    pub cutoff: Option<f32>,

    #[schemars(description = "Resonance (0.0-1.0)")]
    pub resonance: Option<f32>,

    #[schemars(description = "Drive amount (1.0-5.0)")]
    pub drive: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetEnvelopeParams {
    #[schemars(description = "Attack time in seconds (0.001-5.0)")]
    pub attack: Option<f32>,

    #[schemars(description = "Decay time in seconds (0.001-5.0)")]
    pub decay: Option<f32>,

    #[schemars(description = "Sustain level (0.0-1.0)")]
    pub sustain: Option<f32>,

    #[schemars(description = "Release time in seconds (0.001-10.0)")]
    pub release: Option<f32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListParametersParams {}

#[tool_router]
impl SynthMcpServer {
    pub fn new() -> Self {
        Self {
            params: Arc::new(RwLock::new(PluginState::default())),
            tool_router: Self::tool_router(),
        }
    }

    pub fn get_state_handle(&self) -> Arc<RwLock<PluginState>> {
        self.params.clone()
    }

    #[tool(
        description = "Get the current state of all synthesizer parameters including oscillators, filter, and envelope settings"
    )]
    async fn get_synth_state(
        &self,
        Parameters(_params): Parameters<GetStateParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.params.read().await;
        let json = serde_json::to_string_pretty(&*state).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Current Synthesizer State:\n\n{}",
            json
        ))]))
    }

    #[tool(description = "Set parameters for one of the three oscillators (1, 2, or 3)")]
    async fn set_oscillator(
        &self,
        Parameters(params): Parameters<SetOscillatorParams>,
    ) -> Result<CallToolResult, McpError> {
        if params.oscillator < 1 || params.oscillator > 3 {
            return Err(McpError {
                code: ErrorCode(-32602),
                message: Cow::from("Oscillator must be 1, 2, or 3"),
                data: None,
            });
        }

        let mut state = self.params.write().await;
        let osc_num = params.oscillator;

        match osc_num {
            1 => {
                if let Some(waveform) = &params.waveform {
                    if !["Sine", "Square", "Triangle", "Sawtooth"].contains(&waveform.as_str()) {
                        return Err(McpError {
                            code: ErrorCode(-32602),
                            message: Cow::from("Invalid waveform"),
                            data: None,
                        });
                    }
                    state.osc1_waveform = waveform.clone();
                }
                if let Some(freq) = params.frequency {
                    state.osc1_frequency = freq.clamp(20.0, 20000.0);
                }
                if let Some(detune) = params.detune {
                    state.osc1_detune = detune.clamp(-100.0, 100.0);
                }
                if let Some(phase) = params.phase {
                    state.osc1_phase = phase.clamp(0.0, 1.0);
                }
                if let Some(gain) = params.gain {
                    state.osc1_gain = gain.clamp(0.0, 1.0);
                }
                if let Some(octave) = params.octave {
                    state.osc1_octave = octave.clamp(-4, 4);
                }
                if let Some(voices) = params.unison_voices {
                    state.osc1_unison_voices = voices.clamp(1, 8);
                }
                if let Some(detune) = params.unison_detune {
                    state.osc1_unison_detune = detune.clamp(0.0, 50.0);
                }
                if let Some(blend) = params.unison_blend {
                    state.osc1_unison_blend = blend.clamp(0.0, 1.0);
                }
                if let Some(volume) = params.unison_volume {
                    state.osc1_unison_volume = volume.clamp(0.0, 1.0);
                }
            }
            2 => {
                if let Some(waveform) = &params.waveform {
                    if !["Sine", "Square", "Triangle", "Sawtooth"].contains(&waveform.as_str()) {
                        return Err(McpError {
                            code: ErrorCode(-32602),
                            message: Cow::from("Invalid waveform"),
                            data: None,
                        });
                    }
                    state.osc2_waveform = waveform.clone();
                }
                if let Some(freq) = params.frequency {
                    state.osc2_frequency = freq.clamp(20.0, 20000.0);
                }
                if let Some(detune) = params.detune {
                    state.osc2_detune = detune.clamp(-100.0, 100.0);
                }
                if let Some(phase) = params.phase {
                    state.osc2_phase = phase.clamp(0.0, 1.0);
                }
                if let Some(gain) = params.gain {
                    state.osc2_gain = gain.clamp(0.0, 1.0);
                }
                if let Some(octave) = params.octave {
                    state.osc2_octave = octave.clamp(-4, 4);
                }
                if let Some(voices) = params.unison_voices {
                    state.osc2_unison_voices = voices.clamp(1, 8);
                }
                if let Some(detune) = params.unison_detune {
                    state.osc2_unison_detune = detune.clamp(0.0, 50.0);
                }
                if let Some(blend) = params.unison_blend {
                    state.osc2_unison_blend = blend.clamp(0.0, 1.0);
                }
                if let Some(volume) = params.unison_volume {
                    state.osc2_unison_volume = volume.clamp(0.0, 1.0);
                }
            }
            3 => {
                if let Some(waveform) = &params.waveform {
                    if !["Sine", "Square", "Triangle", "Sawtooth"].contains(&waveform.as_str()) {
                        return Err(McpError {
                            code: ErrorCode(-32602),
                            message: Cow::from("Invalid waveform"),
                            data: None,
                        });
                    }
                    state.osc3_waveform = waveform.clone();
                }
                if let Some(freq) = params.frequency {
                    state.osc3_frequency = freq.clamp(20.0, 20000.0);
                }
                if let Some(detune) = params.detune {
                    state.osc3_detune = detune.clamp(-100.0, 100.0);
                }
                if let Some(phase) = params.phase {
                    state.osc3_phase = phase.clamp(0.0, 1.0);
                }
                if let Some(gain) = params.gain {
                    state.osc3_gain = gain.clamp(0.0, 1.0);
                }
                if let Some(octave) = params.octave {
                    state.osc3_octave = octave.clamp(-4, 4);
                }
                if let Some(voices) = params.unison_voices {
                    state.osc3_unison_voices = voices.clamp(1, 8);
                }
                if let Some(detune) = params.unison_detune {
                    state.osc3_unison_detune = detune.clamp(0.0, 50.0);
                }
                if let Some(blend) = params.unison_blend {
                    state.osc3_unison_blend = blend.clamp(0.0, 1.0);
                }
                if let Some(volume) = params.unison_volume {
                    state.osc3_unison_volume = volume.clamp(0.0, 1.0);
                }
            }
            _ => unreachable!(),
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Successfully updated oscillator {}",
            osc_num
        ))]))
    }

    #[tool(
        description = "Set filter parameters including mode, cutoff frequency, resonance, and drive"
    )]
    async fn set_filter(
        &self,
        Parameters(params): Parameters<SetFilterParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.params.write().await;

        if let Some(mode) = &params.mode {
            if !["LowPass", "HighPass", "BandPass", "Notch"].contains(&mode.as_str()) {
                return Err(McpError {
                    code: ErrorCode(-32602),
                    message: Cow::from("Invalid filter mode"),
                    data: None,
                });
            }
            state.filter_mode = mode.clone();
        }

        if let Some(cutoff) = params.cutoff {
            state.filter_cutoff = cutoff.clamp(20.0, 20000.0);
        }

        if let Some(resonance) = params.resonance {
            state.filter_resonance = resonance.clamp(0.0, 1.0);
        }

        if let Some(drive) = params.drive {
            state.filter_drive = drive.clamp(1.0, 5.0);
        }

        Ok(CallToolResult::success(vec![Content::text(
            "Successfully updated filter parameters".to_string(),
        )]))
    }

    #[tool(description = "Set ADSR envelope parameters")]
    async fn set_envelope(
        &self,
        Parameters(params): Parameters<SetEnvelopeParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut state = self.params.write().await;

        if let Some(attack) = params.attack {
            state.envelope_attack = attack.clamp(0.001, 5.0);
        }

        if let Some(decay) = params.decay {
            state.envelope_decay = decay.clamp(0.001, 5.0);
        }

        if let Some(sustain) = params.sustain {
            state.envelope_sustain = sustain.clamp(0.0, 1.0);
        }

        if let Some(release) = params.release {
            state.envelope_release = release.clamp(0.001, 10.0);
        }

        Ok(CallToolResult::success(vec![Content::text(
            "Successfully updated envelope parameters".to_string(),
        )]))
    }

    #[tool(
        description = "List all available synthesizer parameters with their valid ranges and current values"
    )]
    async fn list_parameters(
        &self,
        Parameters(_params): Parameters<ListParametersParams>,
    ) -> Result<CallToolResult, McpError> {
        let state = self.params.read().await;

        let info = format!(
            r#"Triple Oscillator Synthesizer Parameters:

OSCILLATOR 1: waveform={}, freq={} Hz, detune={} cents, gain={}, octave={}, voices={}, unison_detune={} cents
OSCILLATOR 2: waveform={}, freq={} Hz, detune={} cents, gain={}, octave={}, voices={}, unison_detune={} cents
OSCILLATOR 3: waveform={}, freq={} Hz, detune={} cents, gain={}, octave={}, voices={}, unison_detune={} cents
FILTER: mode={}, cutoff={} Hz, resonance={}, drive={}
ENVELOPE: attack={} s, decay={} s, sustain={}, release={} s
"#,
            state.osc1_waveform,
            state.osc1_frequency,
            state.osc1_detune,
            state.osc1_gain,
            state.osc1_octave,
            state.osc1_unison_voices,
            state.osc1_unison_detune,
            state.osc2_waveform,
            state.osc2_frequency,
            state.osc2_detune,
            state.osc2_gain,
            state.osc2_octave,
            state.osc2_unison_voices,
            state.osc2_unison_detune,
            state.osc3_waveform,
            state.osc3_frequency,
            state.osc3_detune,
            state.osc3_gain,
            state.osc3_octave,
            state.osc3_unison_voices,
            state.osc3_unison_detune,
            state.filter_mode,
            state.filter_cutoff,
            state.filter_resonance,
            state.filter_drive,
            state.envelope_attack,
            state.envelope_decay,
            state.envelope_sustain,
            state.envelope_release
        );

        Ok(CallToolResult::success(vec![Content::text(info)]))
    }
}

#[tool_handler]
impl ServerHandler for SynthMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "triple-osc-synth-mcp".to_string(),
                title: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "MCP server for controlling a triple oscillator synthesizer plugin. \
                 Provides tools to read and modify oscillator, filter, and envelope parameters."
                    .to_string(),
            ),
        }
    }
}

pub fn start_mcp_server(state_handle: Arc<RwLock<PluginState>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut server = SynthMcpServer::new();
            server.params = state_handle;

            match server.serve(rmcp::transport::stdio()).await {
                Ok(service) => {
                    if let Err(e) = service.waiting().await {
                        eprintln!("MCP server error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to start MCP server: {}", e);
                }
            }
        });
    })
}
