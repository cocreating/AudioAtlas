//! Regenerable, content-addressed peak envelopes; never send PCM over IPC.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use symphonia::core::{
    audio::SampleBuffer, errors::Error, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint,
};

const VERSION: u32 = 1;
const MAX_BINS: usize = 2048;
const MAX_CACHE_BYTES: u64 = 2 * 1024 * 1024;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Level {
    pub frames_per_bin: u64,
    pub peaks: Vec<[f32; 2]>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Waveform {
    pub version: u32,
    pub sha256: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub frames: u64,
    pub levels: Vec<Level>,
}
pub struct Waveforms {
    directory: PathBuf,
    generation: AtomicU64,
    gate: Mutex<()>,
}
impl Waveforms {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            generation: AtomicU64::new(0),
            gate: Mutex::new(()),
        }
    }
    pub fn begin(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }
    pub fn cancel(&self) {
        self.begin();
    }
    pub fn get(&self, path: &Path, ticket: u64) -> Result<Waveform, String> {
        let _gate = self.gate.lock().map_err(|e| e.to_string())?;
        let started = Instant::now();
        let check = || {
            if self.generation.load(Ordering::SeqCst) != ticket {
                Err("Generación de forma de onda cancelada".to_string())
            } else if started.elapsed() > Duration::from_secs(120) {
                Err("La forma de onda superó el límite de 120 segundos".to_string())
            } else {
                Ok(())
            }
        };
        check()?;
        let before = stamp(path)?;
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buffer = [0; 65536];
        loop {
            check()?;
            let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        let sha256 = format!("{:x}", hash.finalize());
        if stamp(path)? != before {
            return Err("El audio cambió durante el análisis; vuelve a intentarlo".into());
        }
        fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        let cache = self
            .directory
            .join(format!("waveform-v{VERSION}-{sha256}.json"));
        if let Ok(metadata) = fs::metadata(&cache) {
            if metadata.len() <= MAX_CACHE_BYTES {
                if let Ok(bytes) = fs::read(&cache) {
                    if let Ok(wave) = serde_json::from_slice::<Waveform>(&bytes) {
                        if valid(&wave, &sha256) {
                            check()?;
                            if stamp(path)? == before {
                                return Ok(wave);
                            }
                        }
                    }
                }
            }
        }
        let mut wave = decode(path, &check)?;
        wave.sha256 = sha256;
        check()?;
        if stamp(path)? != before {
            return Err("El audio cambió durante el análisis; vuelve a intentarlo".into());
        }
        let bytes = serde_json::to_vec(&wave).map_err(|e| e.to_string())?;
        let mut temp =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        temp.persist(cache).map_err(|e| e.to_string())?;
        // Only our own cache files are evicted. Originals and exports never enter this directory.
        let mut entries = fs::read_dir(&self.directory)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with("waveform-v")
                    && e.path().extension().is_some_and(|s| s == "json")
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        let excess = entries.len().saturating_sub(128);
        for entry in entries.into_iter().take(excess) {
            let _ = fs::remove_file(entry.path());
        }
        Ok(wave)
    }
}
fn valid(w: &Waveform, hash: &str) -> bool {
    w.version == VERSION
        && w.sha256 == hash
        && w.sample_rate > 0
        && (1..=32).contains(&w.channels)
        && w.frames > 0
        && !w.levels.is_empty()
        && w.levels.len() <= 12
        && w.levels.iter().all(|l| {
            l.frames_per_bin > 0
                && !l.peaks.is_empty()
                && l.peaks.len() <= MAX_BINS
                && l.peaks
                    .iter()
                    .all(|p| p[0].is_finite() && p[1].is_finite() && p[0] <= p[1])
        })
}
fn stamp(path: &Path) -> Result<String, String> {
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!(
            "{}:{}:{}:{}:{}:{}",
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
            m.ino()
        ))
    }
    #[cfg(not(unix))]
    {
        Ok(format!(
            "{}:{:?}",
            m.len(),
            m.modified().map_err(|e| e.to_string())?
        ))
    }
}
struct Envelope {
    peaks: Vec<[f32; 2]>,
    span: u64,
    frames: u64,
}
impl Envelope {
    fn new() -> Self {
        Self {
            peaks: Vec::new(),
            span: 256,
            frames: 0,
        }
    }
    fn push(&mut self, samples: &[f32]) {
        if self.frames / self.span >= MAX_BINS as u64 {
            self.peaks = merge(&self.peaks);
            self.span *= 2;
        }
        let index = (self.frames / self.span) as usize;
        if index == self.peaks.len() {
            self.peaks.push([0.0, 0.0]);
        }
        for &sample in samples {
            if sample.is_finite() {
                self.peaks[index][0] = self.peaks[index][0].min(sample);
                self.peaks[index][1] = self.peaks[index][1].max(sample);
            }
        }
        self.frames += 1;
    }
    fn levels(self) -> Vec<Level> {
        let mut levels = vec![Level {
            frames_per_bin: self.span,
            peaks: self.peaks,
        }];
        while levels.last().unwrap().peaks.len() > 64 {
            let last = levels.last().unwrap();
            levels.push(Level {
                frames_per_bin: last.frames_per_bin * 2,
                peaks: merge(&last.peaks),
            });
        }
        levels
    }
}
fn merge(peaks: &[[f32; 2]]) -> Vec<[f32; 2]> {
    peaks
        .chunks(2)
        .map(|p| {
            if p.len() == 2 {
                [p[0][0].min(p[1][0]), p[0][1].max(p[1][1])]
            } else {
                p[0]
            }
        })
        .collect()
}
fn decode(path: &Path, check: &impl Fn() -> Result<(), String>) -> Result<Waveform, String> {
    let source = MediaSourceStream::new(
        Box::new(File::open(path).map_err(|e| e.to_string())?),
        Default::default(),
    );
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|x| x.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| e.to_string())?;
    let track = probed
        .format
        .default_track()
        .ok_or("No hay pista de audio")?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .map_err(|e| e.to_string())?;
    let mut envelope = Envelope::new();
    let mut spec = None;
    let mut pcm = None;
    loop {
        check()?;
        let packet = match probed.format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.to_string()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let audio = decoder.decode(&packet).map_err(|e| e.to_string())?;
        let current = *audio.spec();
        let channels = current.channels.count();
        if channels == 0 || channels > 32 || current.rate == 0 || audio.capacity() > 1_048_576 {
            return Err("Configuración de audio no compatible con la forma de onda".into());
        }
        if spec.is_some_and(|s| s != current) {
            return Err("El formato cambia dentro del archivo".into());
        }
        spec = Some(current);
        let samples =
            pcm.get_or_insert_with(|| SampleBuffer::<f32>::new(audio.capacity() as u64, current));
        if samples.capacity() < audio.capacity() * channels {
            *samples = SampleBuffer::<f32>::new(audio.capacity() as u64, current);
        }
        samples.copy_interleaved_ref(audio);
        for frame in samples.samples().chunks_exact(channels) {
            envelope.push(frame);
        }
    }
    let spec = spec.ok_or("No se pudieron leer muestras de audio")?;
    let frames = envelope.frames;
    if frames == 0 {
        return Err("El archivo no contiene audio".into());
    }
    Ok(Waveform {
        version: VERSION,
        sha256: String::new(),
        sample_rate: spec.rate,
        channels: spec.channels.count(),
        frames,
        levels: envelope.levels(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(path: &Path, negative: bool) {
        let mut w = hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 8000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for i in 0..8000 {
            w.write_sample(if i == 7000 {
                if negative {
                    -16000i16
                } else {
                    16000i16
                }
            } else {
                0i16
            })
            .unwrap();
        }
        w.finalize().unwrap();
    }
    #[test]
    fn waveform_tracks_real_peaks_invalidates_content_and_recovers_corrupt_cache() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("音 ñ.wav");
        fixture(&path, false);
        let service = Waveforms::new(temp.path().join("cache"));
        let first = service.get(&path, service.begin()).unwrap();
        assert_eq!(first.frames, 8000);
        assert_eq!(first.levels[0].peaks[7000 / 256][1], 16000.0 / 32768.0);
        let cache = service
            .directory
            .join(format!("waveform-v1-{}.json", first.sha256));
        fs::write(&cache, "broken").unwrap();
        let rebuilt = service.get(&path, service.begin()).unwrap();
        assert_eq!(rebuilt.sha256, first.sha256);
        fixture(&path, true);
        let changed = service.get(&path, service.begin()).unwrap();
        assert_ne!(changed.sha256, first.sha256);
        assert!(changed.levels[0].peaks.iter().any(|p| p[0] < 0.0));
        let stale = service.begin();
        service.cancel();
        assert!(service.get(&path, stale).is_err());
    }
    #[test]
    fn long_envelopes_remain_bounded_and_preserve_extremes() {
        let mut e = Envelope::new();
        for i in 0..2_000_000 {
            e.push(&[if i == 1_100_000 { -0.9 } else { 0.1 }]);
        }
        assert!(e.peaks.len() <= MAX_BINS);
        let levels = e.levels();
        assert!(levels.len() > 1);
        for level in levels {
            assert!(level.peaks.iter().any(|p| p[0] == -0.9));
        }
    }
    #[test]
    fn corrupt_audio_does_not_create_a_success_cache() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("broken.wav");
        fs::write(&path, "not audio").unwrap();
        let service = Waveforms::new(temp.path().join("cache"));
        assert!(service.get(&path, service.begin()).is_err());
        assert_eq!(fs::read_dir(service.directory).unwrap().count(), 0);
    }
}
