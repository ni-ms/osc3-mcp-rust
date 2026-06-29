//! Decode an uploaded audio file to a mono `f32` buffer via `symphonia`.
//!
//! This allocates heavily and is only ever called off the audio thread (from the
//! Audio Copy panel's background task).

use std::path::Path;
use std::sync::Arc;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Hard cap on decoded length. Keeps separation/analysis bounded and the Gemini
/// payload small; longer clips are truncated.
const MAX_SECONDS: f32 = 30.0;

/// A decoded, downmixed-to-mono audio clip.
#[derive(Clone)]
pub struct Clip {
    pub samples: Arc<Vec<f32>>,
    pub sample_rate: u32,
    pub name: String,
    pub duration_secs: f32,
}

/// Decode `path` to a mono clip. Returns a human-readable error on any failure.
pub fn load_clip(path: &Path) -> Result<Clip, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open file: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("unsupported or unreadable audio: {e}"))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "no audio track in file".to_string())?;
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("no decoder for this format: {e}"))?;

    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(44_100).max(1);
    let mut samples: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            // End of stream (symphonia signals EOF as an IO error) — done.
            Err(SymphoniaError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(format!("read error: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                sample_rate = spec.rate.max(1);
                let channels = spec.channels.count().max(1);

                // Lazily size the interleaved scratch buffer to the decoder's
                // block capacity (constant per codec).
                if sample_buf.is_none() {
                    sample_buf = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
                }

                if let Some(buf) = sample_buf.as_mut() {
                    buf.copy_interleaved_ref(decoded);
                    // Downmix interleaved frames to mono by averaging channels.
                    for frame in buf.samples().chunks(channels) {
                        let sum: f32 = frame.iter().sum();
                        samples.push(sum / channels as f32);
                    }
                }

                if samples.len() as f32 / sample_rate as f32 >= MAX_SECONDS {
                    samples.truncate((MAX_SECONDS * sample_rate as f32) as usize);
                    break;
                }
            }
            // A single corrupt packet shouldn't abort the whole decode.
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(format!("decode error: {e}")),
        }
    }

    if samples.is_empty() {
        return Err("decoded zero samples (empty or silent file)".to_string());
    }

    let duration_secs = samples.len() as f32 / sample_rate as f32;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("clip")
        .to_string();

    Ok(Clip {
        samples: Arc::new(samples),
        sample_rate,
        name,
        duration_secs,
    })
}
