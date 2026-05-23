# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A polyphonic triple-oscillator synthesizer plugin written in Rust on top of
[`nih_plug`](https://github.com/robbert-vdh/nih-plug) (audio/plugin framework) with a
[`vizia_plug`](https://github.com/vizia/vizia-plug) GUI. It builds as **CLAP + VST3**
plugins and as a **standalone application**. Edition is Rust 2024; `nih_plug`/`vizia_plug`/
`rmcp` are pulled from git, so the toolchain must be recent.

There is an in-progress AI-assist feature (Gemini via MCP-style tool calls) — see "AI layer
is currently inert" below before touching it.

## Commands

```sh
# Build the plugin bundle (CLAP + VST3) -> target/bundled/
cargo xtask bundle osc3-mcp-rust --release

# Run the standalone host (opens the GUI in a window with audio I/O)
cargo run                 # debug; cargo run --release for realtime-usable audio

# Type-check / compile without bundling
cargo build
cargo clippy

cargo fmt
```

- The plugin lives in the `[lib]` (`crate-type = ["cdylib", "lib"]`); `src/main.rs` is an
  auto-detected binary that exports the standalone host via `nih_export_standalone`.
- There is **no test suite** in this repo. Don't invent `cargo test` workflows; verify audio
  changes by running the standalone or loading the bundle in a host (the README notes it's
  used in FL Studio).
- Cross-compiling to Windows uses the GNU target with `x86_64-w64-mingw32-gcc` as the linker
  (configured in `Cargo.toml`).

## Real-time safety constraint (important)

`nih_plug` is built with the **`assert_process_allocs`** feature. Any heap allocation on the
audio thread (inside `SineSynth::process` and everything it calls) will **panic at runtime**.
Keep `process()` and the DSP types allocation-free: no `Vec`/`String`/`Box`/format!/locks
that allocate. Allocate up front in `Default`/`initialize` instead. This is why the voice
pool is pre-sized in `SineSynth::default`.

## Architecture

The signal flow and ownership span several files; the big picture:

**Parameters are the single source of truth and are thread-safe.** `SineParams` (in
`params.rs`) is the `#[derive(Params)]` struct. The three oscillators share one
`OscillatorParams` definition nested via `#[nested(id_prefix = "osc1", ...)]`; filter and
envelope are likewise `FilterParams`/`AdsrParams` sub-structs. `nih_plug` stores values in
atomics, so `Arc<SineParams>` is shared freely between the GUI and audio threads without
locks. The GUI never keeps its own copy of param values.

**Audio path** (`lib.rs` + `dsp/`): `SineSynth` owns `Arc<SineParams>` and a fixed
`Vec<Voice>` (`NUM_VOICES` = 16). `process()` handles MIDI (voice allocation / oldest-voice
stealing via `Voice::age`), syncs unison voice counts once per block, then per output sample
builds a `FrameParams` snapshot and sums all active voices. The DSP primitives live in `dsp/`
(`oscillator.rs`, `filter.rs`, `envelope.rs`, `voice.rs`) and are pure `f32` math with no
`nih_plug` dependency, each voice running `UnisonOscillator ×3 → BiquadFilter → Envelope`.

> **Smoothers must be advanced exactly once per sample.** `FrameParams::next` (in
> `dsp/voice.rs`) calls every `param.smoothed.next()` once and the resulting snapshot is
> shared across all voices. Do **not** call `.smoothed.next()` inside the per-voice loop — N
> active voices would advance each smoother N× per sample (this was a fixed bug).

**GUI** (`editor.rs`): a `vizia` editor built by `create_vizia_editor`. The single Vizia
model is `Data { params: Arc<SineParams> }` (`#[derive(Lens)]`). Views read params through
the `Data::params` lens and write them by emitting `RawParamEvent::{BeginSetParameter,
SetParameterNormalized, EndSetParameter}` — this is the idiom that gives the host proper
automation gestures. Layout is tab-based (oscillators / envelope / filter / AI).

**Custom widgets**:
- `knob.rs` — `ParamKnob`, built on `ParamWidgetBase`; handles drag/scroll/double-click-to-default and calls `begin/end_set_parameter`.
- `tab_switcher.rs` — `TabSwitcher` + `TabSwitcherData` model; reusable tab bar with its own event enum.

Each widget module currently injects its own CSS via `cx.add_stylesheet` and the editor has a
large inline `UI_STYLESHEET` const; styling is CSS-string driven, not Rust-typed.

**AI layer is currently inert** (`ai/mcp.rs`, `ai/chat_ui.rs`): `chat_ui.rs` is entirely
commented out, and `start_mcp_server` only spawns a thread that sleeps. `PluginState` in
`ai/mcp.rs` is a hand-maintained *mirror* of `SineParams` behind a `tokio::sync::RwLock`
and is **not wired to the audio params** — changing it does not affect sound (the audio
thread can't read an async lock). If you work on the AI feature, do not extend the mirror;
route AI parameter writes through the real `SineParams` (e.g. a lock-free command queue
drained in `process()`, or `nih_plug`'s `BackgroundTask`/`AsyncExecutor`). See
`ARCHITECTURE_REVIEW.md` for the detailed rationale.

## Conventions

- Plugin identity is set in the `Plugin`/`Vst3Plugin`/`ClapPlugin` impls in `lib.rs`
  (`VST3_CLASS_ID`, `CLAP_ID`, etc.). The VST3 class ID is a fixed 16-byte string — changing
  it breaks host project compatibility.
- Param `#[id]` strings are the host's stable handle for automation/presets; renaming one
  breaks saved state. Treat them as a public API.
- `xtask/` is the standard `nih_plug_xtask` build helper; `cargo xtask bundle <name>` is the
  only supported way to produce distributable plugin bundles.
