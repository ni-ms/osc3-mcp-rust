//! Tool definitions exposed to the model (as Gemini `functionDeclarations`) and
//! the in-plugin dispatcher that executes a tool call.

use crate::ai::{bridge, preset};
use crate::SineParams;
use serde_json::{json, Value};
use vizia_plug::vizia::prelude::*;

use super::chat_ui::ChatEvent;

/// The tool schema sent to Gemini under `tools: [{ functionDeclarations: [...] }]`.
pub fn gemini_tools() -> Value {
    json!([{
        "functionDeclarations": [
            {
                "name": "get_state",
                "description": "Return the current value of every synth parameter as JSON. Call this first when asked to tweak or describe the current sound.",
                "parameters": { "type": "object", "properties": {} }
            },
            {
                "name": "set_parameter",
                "description": concat!(
                    "Set one synth parameter. Call repeatedly to design a sound. Valid names and ranges:\n",
                    "  Oscillators (N = 1, 2, 3): waveformN (sine|square|triangle|sawtooth), ",
                    "frequencyN (20-20000 Hz), detuneN (-100..100 cents), phaseN (0..1), ",
                    "gainN (linear 0.015..1.0), octaveN (-4..4), unison_voicesN (1..8), ",
                    "unison_detuneN (0..50 cents), unison_blendN (0..1), unison_volumeN (0..1).\n",
                    "  Filter: filter_mode (lowpass|highpass|bandpass|notch), filter_cutoff (20-20000 Hz), ",
                    "filter_resonance (0..1), filter_drive (1..5).\n",
                    "  Envelope: attack/decay (0.001..5 s), sustain (0..1), release (0.001..10 s)."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "parameter": { "type": "string", "description": "Exact parameter name, e.g. 'filter_cutoff'." },
                        "value": { "type": "string", "description": "Value: a waveform/mode name for those params, otherwise a number." }
                    },
                    "required": ["parameter", "value"]
                }
            },
            {
                "name": "save_preset",
                "description": "Save the current sound as a named preset file on disk.",
                "parameters": {
                    "type": "object",
                    "properties": { "name": { "type": "string", "description": "Preset name." } },
                    "required": ["name"]
                }
            },
            {
                "name": "load_preset",
                "description": "Load a saved preset by name, applying all of its parameters.",
                "parameters": {
                    "type": "object",
                    "properties": { "name": { "type": "string", "description": "Preset name (see list_presets)." } },
                    "required": ["name"]
                }
            },
            {
                "name": "list_presets",
                "description": "List the names of all saved presets.",
                "parameters": { "type": "object", "properties": {} }
            }
        ]
    }])
}

/// Execute a single tool call in-plugin. Parameter writes are emitted as
/// `RawParamEvent`s through `proxy`; the returned `Value` is fed back to the
/// model as the tool's `functionResponse`.
pub fn dispatch(proxy: &mut ContextProxy, params: &SineParams, name: &str, args: &Value) -> Value {
    match name {
        "get_state" => bridge::read_state(params),

        "set_parameter" => {
            let pname = args.get("parameter").and_then(|v| v.as_str());
            let value = args.get("value");
            let (Some(pname), Some(value)) = (pname, value) else {
                return json!({ "error": "set_parameter requires 'parameter' and 'value'" });
            };

            let result = {
                let mut emit = |ev| {
                    let _ = proxy.emit(ev);
                };
                bridge::apply_write(params, pname, value, &mut emit)
            };

            match result {
                Ok(()) => {
                    let _ = proxy.emit(ChatEvent::ToolLog(format!("🎛 {pname} → {value}")));
                    json!({ "status": "ok", "parameter": pname })
                }
                Err(e) => json!({ "error": e }),
            }
        }

        "save_preset" => {
            let nm = args.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled");
            match preset::save(params, nm) {
                Ok(_) => {
                    let _ = proxy.emit(ChatEvent::ToolLog(format!("💾 saved preset '{nm}'")));
                    json!({ "status": "saved", "name": nm })
                }
                Err(e) => json!({ "error": e }),
            }
        }

        "load_preset" => {
            let nm = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            match preset::load(nm) {
                Ok(data) => {
                    {
                        let mut emit = |ev| {
                            let _ = proxy.emit(ev);
                        };
                        data.apply(params, &mut emit);
                    }
                    let _ = proxy.emit(ChatEvent::ToolLog(format!("📂 loaded preset '{nm}'")));
                    json!({ "status": "loaded", "name": nm })
                }
                Err(e) => json!({ "error": e }),
            }
        }

        "list_presets" => json!({ "presets": preset::list() }),

        _ => json!({ "error": format!("unknown tool '{name}'") }),
    }
}
