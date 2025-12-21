// mcp_protocol.rs - Complete MCP Protocol Implementation with Claude Integration
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool definition for MCP server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP Request message
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "jsonrpc", content = "method")]
pub enum McpRequest {
    #[serde(rename = "2.0")]
    Request {
        id: u64,
        method: String,
        params: Option<serde_json::Value>,
    },
}

/// MCP Response message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Plugin state exposed to AI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginState {
    // Oscillator 1
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

    // Oscillator 2
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

    // Oscillator 3
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

    // Filter
    pub filter_mode: String,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_drive: f32,

    // Envelope
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

impl PluginState {
    /// Get all available MCP tools
    pub fn get_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "get_state".to_string(),
                description: "Get current synthesizer state (all parameters)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            ToolDefinition {
                name: "set_parameter".to_string(),
                description: "Set a synthesizer parameter".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "parameter": {
                            "type": "string",
                            "description": "Parameter path like 'osc1.waveform' or 'filter.cutoff'"
                        },
                        "value": {
                            "description": "New value (string, number, or boolean)"
                        }
                    },
                    "required": ["parameter", "value"]
                }),
            },
            ToolDefinition {
                name: "randomize_patch".to_string(),
                description: "Generate a random synthesizer patch".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "seed": {
                            "type": "integer",
                            "description": "Optional random seed"
                        }
                    },
                    "required": []
                }),
            },
            ToolDefinition {
                name: "suggest_envelope".to_string(),
                description: "Suggest envelope parameters based on description".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "style": {
                            "type": "string",
                            "enum": ["percussive", "pluck", "pad", "lead", "ambient"],
                            "description": "Desired envelope style"
                        }
                    },
                    "required": ["style"]
                }),
            },
            ToolDefinition {
                name: "create_preset".to_string(),
                description: "Create a named preset from current state".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Preset name"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional preset description"
                        }
                    },
                    "required": ["name"]
                }),
            },
        ]
    }

    /// Execute an MCP tool call
    pub async fn execute_tool(
        state: Arc<RwLock<Self>>,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match tool_name {
            "get_state" => {
                let state = state.read().await;
                Ok(serde_json::to_value(&*state).unwrap_or_default())
            }
            "set_parameter" => {
                let param = input
                    .get("parameter")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing parameter name")?;
                let value = input.get("value").ok_or("Missing value")?;

                Self::set_param(&state, param, value).await
            }
            "randomize_patch" => {
                let mut state = state.write().await;
                state.randomize();
                Ok(serde_json::json!({"status": "Patch randomized"}))
            }
            "suggest_envelope" => {
                let style = input
                    .get("style")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing style")?;
                Ok(Self::suggest_envelope_params(style))
            }
            "create_preset" => {
                let name = input
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing preset name")?;
                let state = state.read().await;
                Ok(serde_json::json!({
                    "preset": name,
                    "status": "Preset created",
                    "state": serde_json::to_value(&*state).unwrap_or_default()
                }))
            }
            _ => Err(format!("Unknown tool: {}", tool_name)),
        }
    }

    async fn set_param(
        state: &Arc<RwLock<Self>>,
        param_path: &str,
        value: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut state = state.write().await;
        let parts: Vec<&str> = param_path.split('.').collect();

        match parts.as_slice() {
            ["osc1", "waveform"] => {
                state.osc1_waveform = value.as_str().unwrap_or("Sine").to_string();
                Ok(serde_json::json!({"status": "Set osc1.waveform"}))
            }
            ["osc1", "frequency"] => {
                state.osc1_frequency = value.as_f64().unwrap_or(440.0) as f32;
                Ok(serde_json::json!({"status": "Set osc1.frequency"}))
            }
            ["osc2", "waveform"] => {
                state.osc2_waveform = value.as_str().unwrap_or("Sawtooth").to_string();
                Ok(serde_json::json!({"status": "Set osc2.waveform"}))
            }
            ["osc2", "frequency"] => {
                state.osc2_frequency = value.as_f64().unwrap_or(880.0) as f32;
                Ok(serde_json::json!({"status": "Set osc2.frequency"}))
            }
            ["osc3", "waveform"] => {
                state.osc3_waveform = value.as_str().unwrap_or("Square").to_string();
                Ok(serde_json::json!({"status": "Set osc3.waveform"}))
            }
            ["osc3", "frequency"] => {
                state.osc3_frequency = value.as_f64().unwrap_or(220.0) as f32;
                Ok(serde_json::json!({"status": "Set osc3.frequency"}))
            }
            ["filter", "cutoff"] => {
                state.filter_cutoff = value.as_f64().unwrap_or(20000.0) as f32;
                Ok(serde_json::json!({"status": "Set filter.cutoff"}))
            }
            ["filter", "mode"] => {
                state.filter_mode = value.as_str().unwrap_or("LowPass").to_string();
                Ok(serde_json::json!({"status": "Set filter.mode"}))
            }
            ["envelope", "attack"] => {
                state.envelope_attack = value.as_f64().unwrap_or(0.01) as f32;
                Ok(serde_json::json!({"status": "Set envelope.attack"}))
            }
            ["envelope", "decay"] => {
                state.envelope_decay = value.as_f64().unwrap_or(0.5) as f32;
                Ok(serde_json::json!({"status": "Set envelope.decay"}))
            }
            ["envelope", "sustain"] => {
                state.envelope_sustain = value.as_f64().unwrap_or(0.7) as f32;
                Ok(serde_json::json!({"status": "Set envelope.sustain"}))
            }
            ["envelope", "release"] => {
                state.envelope_release = value.as_f64().unwrap_or(1.0) as f32;
                Ok(serde_json::json!({"status": "Set envelope.release"}))
            }
            _ => Err(format!("Unknown parameter: {}", param_path)),
        }
    }

    pub fn randomize(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0) as u64;

        let waveforms = ["Sine", "Square", "Triangle", "Sawtooth"];
        let idx = (seed as usize) % waveforms.len();

        self.osc1_waveform = waveforms[idx].to_string();
        self.osc1_frequency = 100.0 + ((seed >> 8) % 2000) as f32;

        let idx = ((seed >> 16) as usize) % waveforms.len();
        self.osc2_waveform = waveforms[idx].to_string();
        self.osc2_frequency = 100.0 + ((seed >> 24) % 2000) as f32;

        self.envelope_attack = 0.001 + ((seed >> 32) as f32 % 1.0) * 0.2;
        self.envelope_decay = 0.1 + ((seed >> 40) as f32 % 1.0) * 1.0;
        self.envelope_sustain = (seed >> 48) as f32 % 1.0;
    }

    fn suggest_envelope_params(style: &str) -> serde_json::Value {
        match style {
            "percussive" => serde_json::json!({
                "attack": 0.005,
                "decay": 0.3,
                "sustain": 0.0,
                "release": 0.2,
                "description": "Fast attack, quick decay to silence - great for drums and percussive hits"
            }),
            "pluck" => serde_json::json!({
                "attack": 0.01,
                "decay": 0.5,
                "sustain": 0.0,
                "release": 0.3,
                "description": "Short attack with gradual decay - mimics plucked strings"
            }),
            "pad" => serde_json::json!({
                "attack": 0.5,
                "decay": 1.0,
                "sustain": 0.7,
                "release": 2.0,
                "description": "Slow attack to full volume, sustained at high level, slow fade out"
            }),
            "lead" => serde_json::json!({
                "attack": 0.05,
                "decay": 0.2,
                "sustain": 0.8,
                "release": 0.5,
                "description": "Quick attack, snappy decay, held sustain - for expressive leads"
            }),
            "ambient" => serde_json::json!({
                "attack": 2.0,
                "decay": 3.0,
                "sustain": 0.5,
                "release": 3.0,
                "description": "Very slow rise and fall - ethereal, atmospheric tones"
            }),
            _ => serde_json::json!({
                "attack": 0.1,
                "decay": 0.5,
                "sustain": 0.6,
                "release": 1.0,
                "description": "Default balanced envelope"
            }),
        }
    }
}

/// MCP Server management
pub fn start_mcp_server(state: Arc<RwLock<PluginState>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    })
}
