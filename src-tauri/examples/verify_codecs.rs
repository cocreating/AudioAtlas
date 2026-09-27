//! Optional corpus from scripts/create-codec-fixtures.py; no end-user FFmpeg dependency.
use audio_atlas_lib::{
    audio_stream::{BufferedAudio, LoopRegion},
    waveform::Waveforms,
};
use std::{path::PathBuf, sync::atomic::Ordering};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Usage: verify_codecs DIRECTORY")?,
    );
    let temp = tempfile::tempdir()?;
    let waves = Waveforms::new(temp.path().join("cache"));
    for ext in ["wav", "aiff", "flac", "mp3", "m4a", "aac", "ogg"] {
        let path = root.join(format!("reference.{ext}"));
        let wave = waves.get(&path, waves.begin());
        let playback = BufferedAudio::open(path.clone(), 0.0, None);
        let seek = BufferedAudio::open(path.clone(), 1.0, None);
        let looping = BufferedAudio::open(
            path,
            1.0,
            Some(LoopRegion {
                start: 1.0,
                end: 1.2,
            }),
        );
        let status = |r: Result<BufferedAudio, String>| match r {
            Ok(mut s) => {
                let count = s.by_ref().take(512).count();
                assert_eq!(count, 512);
                assert_eq!(s.status.underruns.load(Ordering::Relaxed), 0);
                "OK".to_string()
            }
            Err(e) => format!("UNSUPPORTED: {e}"),
        };
        println!(
            "{ext}: waveform={} / decode={} / seek={} / loop={}",
            wave.map(|w| format!("OK ({} frames)", w.frames))
                .unwrap_or_else(|e| format!("UNSUPPORTED: {e}")),
            status(playback),
            status(seek),
            status(looping)
        );
    }
    Ok(())
}
