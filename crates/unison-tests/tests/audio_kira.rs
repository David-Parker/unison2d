//! Smoke test: the real kira-backed `AudioSystem` can load and play a WAV.
//!
//! Gated behind `--features audio-kira-tests` because it requires an actual
//! audio output device. Also gated to non-wasm: kira's cpal backend only runs
//! on native.

#![cfg(all(not(target_arch = "wasm32"), feature = "audio-kira-tests"))]

use unison_audio::{AudioSystem, KiraBackend, PlayParams};

/// Build a minimal valid 0.1s silent mono 44.1kHz PCM WAV (44-byte header +
/// 4410 samples · 2 bytes = 8820 bytes of data).
fn tiny_wav() -> Vec<u8> {
    let sample_rate: u32 = 44_100;
    let num_samples: u32 = 4_410; // 0.1s
    let bytes_per_sample: u16 = 2;
    let channels: u16 = 1;
    let byte_rate = sample_rate * u32::from(channels) * u32::from(bytes_per_sample);
    let block_align = channels * bytes_per_sample;
    let data_size = num_samples * u32::from(block_align);
    let chunk_size = 36 + data_size;

    let mut out = Vec::with_capacity(44 + data_size as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&chunk_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&(bytes_per_sample * 8).to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend(std::iter::repeat(0u8).take(data_size as usize));
    out
}

#[test]
fn kira_load_play_smoke() {
    let backend = match KiraBackend::new() {
        Ok(b) => b,
        Err(_) => {
            eprintln!("no audio device available; skipping");
            return;
        }
    };
    let mut sys = AudioSystem::with_backend(Box::new(backend));
    let id = sys.load(&tiny_wav()).expect("load");
    let pb = sys
        .play(id, PlayParams::with_bus(sys.sfx_bus()))
        .expect("play");
    assert!(sys.is_playing(pb));
}
