use nih_plug::nih_export_standalone;
use osc3_mcp_rust::SineSynth;

fn main() {

    nih_export_standalone::<SineSynth>();
}
