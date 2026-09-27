pub use crate::audio_stream::LoopRegion;
use crate::audio_stream::{BufferedAudio, StreamStatus};
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{atomic::Ordering, mpsc, Arc},
    time::Duration,
};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub file_id: Option<String>,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
    pub loop_region: Option<LoopRegion>,
    pub underrun_frames: u64,
    pub error: Option<String>,
}
#[derive(Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "camelCase")]
pub enum Control {
    Pause,
    Resume,
    Seek(f64),
    Volume(f32),
    Stop,
    #[serde(rename_all = "camelCase")]
    SetLoop {
        file_id: String,
        region: Option<LoopRegion>,
    },
}
enum Message {
    Play(String, PathBuf, mpsc::Sender<Result<(), String>>),
    Control(Control, mpsc::Sender<Result<(), String>>),
    State(mpsc::Sender<PlayerState>),
}
#[derive(Clone)]
pub struct Playback {
    sender: mpsc::Sender<Message>,
}
struct Engine {
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    status: Option<Arc<StreamStatus>>,
    path: Option<PathBuf>,
    rate: u32,
    state: PlayerState,
}
impl Engine {
    fn snapshot(&mut self) -> PlayerState {
        if let Some(status) = &self.status {
            self.state.position = status.position.load(Ordering::Relaxed) as f64 / self.rate as f64;
            if self.state.duration > 0.0 {
                self.state.position = self.state.position.min(self.state.duration);
            }
            self.state.underrun_frames = status.underruns.load(Ordering::Relaxed);
            self.state.error = status.error.lock().unwrap().clone();
        }
        self.state.playing = self
            .sink
            .as_ref()
            .is_some_and(|s| !s.is_paused() && !s.empty());
        self.state.clone()
    }
    fn load(
        &mut self,
        id: String,
        path: PathBuf,
        start: f64,
        region: Option<LoopRegion>,
        paused: bool,
    ) -> Result<(), String> {
        // Prefill on the decoder worker before replacing a working source.
        let source = BufferedAudio::open(path.clone(), start, region)?;
        if self.stream.is_none() {
            self.stream = Some(
                OutputStreamBuilder::open_default_stream()
                    .map_err(|e| format!("No se pudo abrir la salida de audio: {e}"))?,
            );
        }
        let next = Sink::connect_new(self.stream.as_ref().unwrap().mixer());
        next.pause();
        next.set_volume(self.state.volume);
        self.rate = source.sample_rate();
        self.state.duration = source.duration;
        self.status = Some(source.status.clone());
        if let Some(old) = self.sink.take() {
            old.stop();
        }
        next.append(source);
        if !paused {
            next.play();
        }
        self.sink = Some(next);
        self.path = Some(path);
        self.state.file_id = Some(id);
        self.state.loop_region = region;
        self.state.error = None;
        self.snapshot();
        Ok(())
    }
    fn reload(
        &mut self,
        start: f64,
        region: Option<LoopRegion>,
        paused: bool,
    ) -> Result<(), String> {
        self.load(
            self.state
                .file_id
                .clone()
                .ok_or("Selecciona un audio primero")?,
            self.path.clone().ok_or("Selecciona un audio primero")?,
            start,
            region,
            paused,
        )
    }
    fn control(&mut self, control: Control) -> Result<(), String> {
        let current = self.snapshot();
        match control {
            Control::Volume(volume) => {
                if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
                    return Err("Volumen no válido".into());
                }
                self.state.volume = volume;
                if let Some(s) = &self.sink {
                    s.set_volume(volume);
                }
                Ok(())
            }
            Control::Stop => {
                if let Some(s) = self.sink.take() {
                    s.stop();
                }
                self.status = None;
                self.path = None;
                self.state = PlayerState {
                    volume: self.state.volume,
                    ..Default::default()
                };
                Ok(())
            }
            Control::Pause => {
                self.sink
                    .as_ref()
                    .ok_or("Selecciona un audio primero")?
                    .pause();
                Ok(())
            }
            Control::Resume => {
                let sink = self.sink.as_ref().ok_or("Selecciona un audio primero")?;
                if sink.empty() {
                    self.reload(
                        current.loop_region.map(|r| r.start).unwrap_or(0.0),
                        current.loop_region,
                        false,
                    )
                } else {
                    sink.play();
                    Ok(())
                }
            }
            Control::Seek(seconds) => {
                if !seconds.is_finite() || seconds < 0.0 {
                    return Err("Posición no válida".into());
                }
                let seconds = if current.duration > 0.0 {
                    seconds.min(current.duration)
                } else {
                    seconds
                };
                let paused = self.sink.as_ref().is_none_or(|s| s.is_paused());
                self.reload(seconds, current.loop_region, paused)
            }
            Control::SetLoop { file_id, region } => {
                if current.file_id.as_ref() != Some(&file_id) {
                    return Err("El sonido ha cambiado; vuelve a elegir el loop".into());
                }
                let region = region.map(|r| r.validate(current.duration)).transpose()?;
                let paused = !current.playing;
                self.reload(current.position, region, paused)
            }
        }
    }
}
impl Playback {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut engine = Engine {
                stream: None,
                sink: None,
                status: None,
                path: None,
                rate: 1,
                state: PlayerState {
                    volume: 0.6,
                    ..Default::default()
                },
            };
            while let Ok(message) = receiver.recv() {
                match message {
                    Message::Play(id, path, reply) => {
                        let _ = reply.send(engine.load(id, path, 0.0, None, false));
                    }
                    Message::Control(control, reply) => {
                        let _ = reply.send(engine.control(control));
                    }
                    Message::State(reply) => {
                        let _ = reply.send(engine.snapshot());
                    }
                }
            }
        });
        Self { sender }
    }
    pub fn play(&self, id: String, path: PathBuf) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Message::Play(id, path, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(12))
            .map_err(|e| e.to_string())?
    }
    pub fn control(&self, control: Control) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Message::Control(control, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(12))
            .map_err(|e| e.to_string())?
    }
    pub fn state(&self) -> Result<PlayerState, String> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Message::State(tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(12))
            .map_err(|e| e.to_string())
    }
}
