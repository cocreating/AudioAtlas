//! Decode on a worker; the audio callback only consumes a fixed, lock-free FIFO.
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapRb,
};
use rodio::{Decoder, Source};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

const BUFFER_FRAMES: usize = 16_384;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct LoopRegion {
    pub start: f64,
    pub end: f64,
}
impl LoopRegion {
    pub fn validate(self, duration: f64) -> Result<Self, String> {
        if !self.start.is_finite()
            || !self.end.is_finite()
            || self.start < 0.0
            || self.end - self.start < 0.05
            || self.end > duration
            || !duration.is_finite()
        {
            return Err("El loop debe durar al menos 0,05 s y quedar dentro del audio".into());
        }
        Ok(self)
    }
}
#[derive(Clone, Copy, Default)]
struct Frame {
    samples: [f32; 2],
    position: u64,
}
#[derive(Default)]
pub struct StreamStatus {
    pub position: AtomicU64,
    pub underruns: AtomicU64,
    finished: AtomicBool,
    cancel: AtomicBool,
    pub error: Mutex<Option<String>>,
}
pub struct BufferedAudio {
    consumer: HeapCons<Frame>,
    pub status: Arc<StreamStatus>,
    channels: u16,
    rate: u32,
    pub duration: f64,
    frame: Frame,
    channel: u16,
}
impl BufferedAudio {
    pub fn open(path: PathBuf, start: f64, region: Option<LoopRegion>) -> Result<Self, String> {
        let (mut producer, consumer) = HeapRb::<Frame>::new(BUFFER_FRAMES).split();
        let status = Arc::new(StreamStatus::default());
        let worker_status = status.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut ready = Some(ready_tx);
            let result = (|| -> Result<(), String> {
                let mut decoder = Decoder::try_from(File::open(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                let channels = decoder.channels();
                let rate = decoder.sample_rate();
                if !(1..=2).contains(&channels) || rate == 0 {
                    return Err("La preescucha admite mono y estéreo".into());
                }
                let duration = decoder
                    .total_duration()
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
                let region = region.map(|r| r.validate(duration)).transpose()?;
                if !start.is_finite() || start < 0.0 {
                    return Err("Posición no válida".into());
                }
                let start = region
                    .map(|r| {
                        if start < r.start || start >= r.end {
                            r.start
                        } else {
                            start
                        }
                    })
                    .unwrap_or(start);
                if duration > 0.0 && start > duration {
                    return Err("Posición no válida".into());
                }
                if start > 0.0 {
                    decoder
                        .try_seek(
                            Duration::try_from_secs_f64(start)
                                .map_err(|_| "Posición fuera de rango")?,
                        )
                        .map_err(|e| e.to_string())?;
                }
                let mut frame_index = (start * rate as f64).round() as u64;
                worker_status.position.store(frame_index, Ordering::Relaxed);
                let bounds = region.map(|r| {
                    (
                        (r.start * rate as f64).round() as u64,
                        (r.end * rate as f64).round() as u64,
                    )
                });
                let mut buffered = 0;
                loop {
                    if worker_status.cancel.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    if let Some((a, b)) = bounds {
                        if frame_index >= b {
                            decoder
                                .try_seek(Duration::from_secs_f64(a as f64 / rate as f64))
                                .map_err(|e| e.to_string())?;
                            frame_index = a;
                        }
                    }
                    let Some(first) = decoder.next() else {
                        if bounds.is_some() {
                            return Err("El audio terminó antes del final del loop".into());
                        }
                        break;
                    };
                    let second = if channels == 2 {
                        decoder.next().ok_or("Frame estéreo incompleto")?
                    } else {
                        0.0
                    };
                    frame_index += 1;
                    let mut frame = Frame {
                        samples: [first, second],
                        position: frame_index,
                    };
                    loop {
                        match producer.try_push(frame) {
                            Ok(()) => break,
                            Err(pending) => frame = pending,
                        }
                        if worker_status.cancel.load(Ordering::Relaxed) {
                            return Ok(());
                        }
                        std::thread::sleep(Duration::from_millis(2));
                    }
                    buffered += 1;
                    if buffered == 1024 {
                        if let Some(tx) = ready.take() {
                            let _ = tx.send(Ok((channels, rate, duration)));
                        }
                    }
                }
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Ok((channels, rate, duration)));
                }
                Ok(())
            })();
            if let Err(error) = result {
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Err(error.clone()));
                }
                *worker_status.error.lock().unwrap() = Some(error);
            }
            worker_status.finished.store(true, Ordering::Release);
        });
        let metadata = ready_rx
            .recv_timeout(Duration::from_secs(8))
            .map_err(|e| e.to_string())
            .and_then(|r| r);
        match metadata {
            Ok((channels, rate, duration)) => Ok(Self {
                consumer,
                status,
                channels,
                rate,
                duration,
                frame: Frame::default(),
                channel: 0,
            }),
            Err(error) => {
                status.cancel.store(true, Ordering::Relaxed);
                Err(error)
            }
        }
    }
}
impl Drop for BufferedAudio {
    fn drop(&mut self) {
        self.status.cancel.store(true, Ordering::Relaxed);
    }
}
impl Iterator for BufferedAudio {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.channel == 0 {
            self.frame = match self.consumer.try_pop() {
                Some(frame) => frame,
                None => {
                    if self.status.finished.load(Ordering::Acquire) {
                        // Recheck after the producer's release to avoid dropping its last frame.
                        self.consumer.try_pop()?
                    } else {
                        self.status.underruns.fetch_add(1, Ordering::Relaxed);
                        Frame {
                            samples: [0.0; 2],
                            position: self.status.position.load(Ordering::Relaxed),
                        }
                    }
                }
            };
        }
        let value = self.frame.samples[self.channel as usize];
        self.channel = (self.channel + 1) % self.channels;
        if self.channel == 0 {
            self.status
                .position
                .store(self.frame.position, Ordering::Relaxed);
        }
        Some(value)
    }
}
impl Source for BufferedAudio {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(path: &std::path::Path, channels: u16) {
        let mut w = hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels,
                sample_rate: 8000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for i in 0..8000 {
            w.write_sample(i as i16).unwrap();
            if channels == 2 {
                w.write_sample(-(i as i16)).unwrap();
            }
        }
        w.finalize().unwrap();
    }
    #[test]
    fn loops_exact_frames_without_growing_buffer_and_preserves_channels() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("stereo.wav");
        fixture(&path, 2);
        let mut source = BufferedAudio::open(
            path,
            0.0,
            Some(LoopRegion {
                start: 0.1,
                end: 0.2,
            }),
        )
        .unwrap();
        use ringbuf::traits::Observer;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while source.consumer.occupied_len() < 2400 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(source.consumer.occupied_len() >= 2400);
        for i in 0..2400 {
            let expected = (800 + i % 800) as f32 / 32768.0;
            assert_eq!(source.next(), Some(expected), "frame {i}");
            assert_eq!(source.next(), Some(-expected));
        }
        assert_eq!(source.status.position.load(Ordering::Relaxed), 1600);
    }
    #[test]
    fn empty_fifo_returns_silence_without_advancing_source_time() {
        let (_producer, consumer) = HeapRb::<Frame>::new(2).split();
        let status = Arc::new(StreamStatus::default());
        status.position.store(8000, Ordering::Relaxed);
        let mut source = BufferedAudio {
            consumer,
            status: status.clone(),
            channels: 2,
            rate: 8000,
            duration: 2.0,
            frame: Frame::default(),
            channel: 0,
        };
        for _ in 0..100 {
            assert_eq!(source.next(), Some(0.0));
        }
        assert_eq!(status.position.load(Ordering::Relaxed), 8000);
        assert_eq!(status.underruns.load(Ordering::Relaxed), 50);
        status.finished.store(true, Ordering::Release);
        assert_eq!(source.next(), None);
    }
    #[test]
    fn seeking_starts_at_requested_frame_and_rejects_invalid_ranges() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("mono.wav");
        fixture(&path, 1);
        let mut source = BufferedAudio::open(path.clone(), 0.5, None).unwrap();
        assert_eq!(source.next(), Some(4000.0 / 32768.0));
        assert!(BufferedAudio::open(path.clone(), f64::NAN, None).is_err());
        let mut loop_after_end = BufferedAudio::open(
            path.clone(),
            1.1,
            Some(LoopRegion {
                start: 0.1,
                end: 0.2,
            }),
        )
        .unwrap();
        assert_eq!(loop_after_end.next(), Some(800.0 / 32768.0));
        for range in [
            LoopRegion {
                start: 0.2,
                end: 0.1,
            },
            LoopRegion {
                start: -1.0,
                end: 0.5,
            },
            LoopRegion {
                start: 0.0,
                end: 1.1,
            },
            LoopRegion {
                start: 0.0,
                end: f64::NAN,
            },
        ] {
            assert!(range.validate(1.0).is_err());
        }
    }
}
