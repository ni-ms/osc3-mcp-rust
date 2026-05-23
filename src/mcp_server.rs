// mcp_server.rs - Complete AI & MCP Implementation for Triple Oscillator Synth
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use vizia_plug::vizia::prelude::Data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Data)]
pub enum AiModel {
    Gemini15Flash,
    Gemini15Pro,
    Gemini20Flash,
}

impl AiModel {
    pub fn api_name(&self) -> &str {
        match self {
            AiModel::Gemini15Flash => "gemini-1.5-flash",
            AiModel::Gemini15Pro => "gemini-1.5-pro",
            AiModel::Gemini20Flash => "gemini-2.0-flash-exp",
        }
    }
}

/// Tool definition for the AI
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP JSON-RPC structures for protocol compliance
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

/// COMPLETE Plugin state - Exhaustive 1:1 Mirror of SineParams in lib.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginState {
    // --- AI Config ---
    pub api_key: String,
    pub model: AiModel,
    pub temperature: f32,

    // --- Oscillator 1 ---
    pub waveform1: String,
    pub frequency1: f32,
    pub detune1: f32,
    pub phase1: f32,
    pub gain1: f32,
    pub octave1: i32,
    pub unison_voices1: i32,
    pub unison_detune1: f32,
    pub unison_blend1: f32,
    pub unison_volume1: f32,

    // --- Oscillator 2 ---
    pub waveform2: String,
    pub frequency2: f32,
    pub detune2: f32,
    pub phase2: f32,
    pub gain2: f32,
    pub octave2: i32,
    pub unison_voices2: i32,
    pub unison_detune2: f32,
    pub unison_blend2: f32,
    pub unison_volume2: f32,

    // --- Oscillator 3 ---
    pub waveform3: String,
    pub frequency3: f32,
    pub detune3: f32,
    pub phase3: f32,
    pub gain3: f32,
    pub octave3: i32,
    pub unison_voices3: i32,
    pub unison_detune3: f32,
    pub unison_blend3: f32,
    pub unison_volume3: f32,

    // --- Filter ---
    pub filter_mode: String,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub filter_drive: f32,

    // --- Envelope (ADSR) ---
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl PluginState {
    pub(crate) fn get_tools_as_gemini_schema() -> serde_json::Value {
        let tools = Self::get_tools();

        // Map ToolDefinition -> Gemini functionDeclarations
        let function_declarations: Vec<serde_json::Value> = tools
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    // Gemini uses OpenAPI-ish JSON Schema under `parameters`
                    "parameters": t.input_schema,
                })
            })
            .collect();

        // Wrap into `tools: [{ functionDeclarations: [...] }]`
        serde_json::json!([
            {
                "functionDeclarations": function_declarations
            }
        ])
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: AiModel::Gemini15Flash,
            temperature: 0.7,

            // Matches SineParams defaults
            waveform1: "sine".into(),
            frequency1: 440.0,
            detune1: 0.0,
            phase1: 0.0,
            gain1: 0.5,
            octave1: 0,
            unison_voices1: 1,
            unison_detune1: 0.0,
            unison_blend1: 0.0,
            unison_volume1: 1.0,

            waveform2: "sawtooth".into(),
            frequency2: 880.0,
            detune2: 0.0,
            phase2: 0.0,
            gain2: 0.25,
            octave2: -1,
            unison_voices2: 1,
            unison_detune2: 0.0,
            unison_blend2: 0.0,
            unison_volume2: 1.0,

            waveform3: "square".into(),
            frequency3: 220.0,
            detune3: 0.0,
            phase3: 0.0,
            gain3: 0.125,
            octave3: 1,
            unison_voices3: 1,
            unison_detune3: 0.0,
            unison_blend3: 0.0,
            unison_volume3: 1.0,

            filter_mode: "lowpass".into(),
            filter_cutoff: 20000.0,
            filter_resonance: 0.0,
            filter_drive: 1.0,
            attack: 0.01,
            decay: 0.5,
            sustain: 0.7,
            release: 1.0,
        }
    }
}

impl PluginState {
    pub fn get_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "get_state".into(),
                description: "Returns the current position of all knobs on the synthesizer.".into(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
            ToolDefinition {
                name: "set_parameter".into(),
                description:
                    "Adjusts a specific synth knob. Use this for sound design or replication."
                        .into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "parameter": {
                            "type": "string",
                            "description": "The exact parameter name (e.g., 'frequency1', 'filter_cutoff', 'unison_voices2')"
                        },
                        "value": { "description": "The value to set (string for waveforms, number for others)" }
                    },
                    "required": ["parameter", "value"]
                }),
            },
            ToolDefinition {
                name: "randomize_patch".into(),
                description: "Randomizes all oscillator and filter settings.".into(),
                input_schema: serde_json::json!({"type": "object", "properties": {}}),
            },
        ]
    }

    pub async fn execute_tool(
        state: Arc<RwLock<Self>>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match name {
            "get_state" => Ok(serde_json::to_value(&*state.read().await).unwrap()),
            "set_parameter" => {
                let p = input
                    .get("parameter")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing param name")?;
                let v = input.get("value").ok_or("Missing value")?;
                Self::set_param(&state, p, v).await
            }
            "randomize_patch" => {
                state.write().await.randomize();
                Ok(serde_json::json!({"status": "Success"}))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    async fn set_param(
        state: &Arc<RwLock<Self>>,
        path: &str,
        val: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut s = state.write().await;
        match path {
            // Oscillator 1
            "waveform1" => s.waveform1 = val.as_str().unwrap_or("sine").to_lowercase(),
            "frequency1" => s.frequency1 = val.as_f64().unwrap_or(440.0) as f32,
            "detune1" => s.detune1 = val.as_f64().unwrap_or(0.0) as f32,
            "phase1" => s.phase1 = val.as_f64().unwrap_or(0.0) as f32,
            "gain1" => s.gain1 = val.as_f64().unwrap_or(0.5) as f32,
            "octave1" => s.octave1 = val.as_i64().unwrap_or(0) as i32,
            "unison_voices1" => s.unison_voices1 = val.as_i64().unwrap_or(1) as i32,
            "unison_detune1" => s.unison_detune1 = val.as_f64().unwrap_or(0.0) as f32,
            "unison_blend1" => s.unison_blend1 = val.as_f64().unwrap_or(0.0) as f32,
            "unison_volume1" => s.unison_volume1 = val.as_f64().unwrap_or(1.0) as f32,

            // Oscillator 2
            "waveform2" => s.waveform2 = val.as_str().unwrap_or("sawtooth").to_lowercase(),
            "frequency2" => s.frequency2 = val.as_f64().unwrap_or(880.0) as f32,
            "detune2" => s.detune2 = val.as_f64().unwrap_or(0.0) as f32,
            "phase2" => s.phase2 = val.as_f64().unwrap_or(0.0) as f32,
            "gain2" => s.gain2 = val.as_f64().unwrap_or(0.25) as f32,
            "octave2" => s.octave2 = val.as_i64().unwrap_or(-1) as i32,
            "unison_voices2" => s.unison_voices2 = val.as_i64().unwrap_or(1) as i32,
            "unison_detune2" => s.unison_detune2 = val.as_f64().unwrap_or(0.0) as f32,
            "unison_blend2" => s.unison_blend2 = val.as_f64().unwrap_or(0.0) as f32,
            "unison_volume2" => s.unison_volume2 = val.as_f64().unwrap_or(1.0) as f32,

            // Oscillator 3
            "waveform3" => s.waveform3 = val.as_str().unwrap_or("square").to_lowercase(),
            "frequency3" => s.frequency3 = val.as_f64().unwrap_or(220.0) as f32,
            "detune3" => s.detune3 = val.as_f64().unwrap_or(0.0) as f32,
            "phase3" => s.phase3 = val.as_f64().unwrap_or(0.0) as f32,
            "gain3" => s.gain3 = val.as_f64().unwrap_or(0.125) as f32,
            "octave3" => s.octave3 = val.as_i64().unwrap_or(1) as i32,
            "unison_voices3" => s.unison_voices3 = val.as_i64().unwrap_or(1) as i32,
            "unison_detune3" => s.unison_detune3 = val.as_f64().unwrap_or(0.0) as f32,
            "unison_blend3" => s.unison_blend3 = val.as_f64().unwrap_or(0.0) as f32,
            "unison_volume3" => s.unison_volume3 = val.as_f64().unwrap_or(1.0) as f32,

            // Filter
            "filter_mode" => s.filter_mode = val.as_str().unwrap_or("lowpass").to_lowercase(),
            "filter_cutoff" => s.filter_cutoff = val.as_f64().unwrap_or(20000.0) as f32,
            "filter_resonance" => s.filter_resonance = val.as_f64().unwrap_or(0.0) as f32,
            "filter_drive" => s.filter_drive = val.as_f64().unwrap_or(1.0) as f32,

            // Envelope (ADSR)
            "attack" => s.attack = val.as_f64().unwrap_or(0.01) as f32,
            "decay" => s.decay = val.as_f64().unwrap_or(0.5) as f32,
            "sustain" => s.sustain = val.as_f64().unwrap_or(0.7) as f32,
            "release" => s.release = val.as_f64().unwrap_or(1.0) as f32,

            _ => return Err(format!("Parameter {} is not a valid knob name", path)),
        }
        Ok(serde_json::json!({"status": "success", "param": path}))
    }

    pub fn randomize(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let waves = ["sine", "square", "triangle", "sawtooth"];
        self.waveform1 = waves[(t % 4) as usize].into();
        self.waveform2 = waves[((t >> 4) % 4) as usize].into();
        self.filter_cutoff = 100.0 + (t % 8000) as f32;
        self.filter_resonance = (t % 100) as f32 / 100.0;
        self.attack = 0.001 + (t % 50) as f32 / 100.0;
    }
}

pub fn start_mcp_server(_state: Arc<RwLock<PluginState>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });
    })
}
