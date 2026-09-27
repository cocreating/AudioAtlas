use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf, sync::mpsc, time::Duration};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub file_id: Option<String>,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f32,
}
#[derive(Deserialize)]
#[serde(tag = "action", content = "value", rename_all = "camelCase")]
pub enum Control {
    Pause,
    Resume,
    Seek(f64),
    Volume(f32),
    Stop,
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
impl Playback {
    pub fn start() -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stream = None;
            let mut sink: Option<Sink> = None;
            let mut state = PlayerState {
                volume: 0.6,
                ..Default::default()
            };
            while let Ok(message) = receiver.recv() {
                match message {
                    Message::Play(id, path, reply) => {
                        let result = (|| -> Result<(), String> {
                            let decoder =
                                Decoder::try_from(File::open(path).map_err(|e| e.to_string())?)
                                    .map_err(|e| e.to_string())?;
                            // Avoid an unverified multichannel downmix in the first vertical.
                            if decoder.channels() > 2 {
                                return Err("La preescucha inicial admite mono y estéreo".into());
                            }
                            let duration = decoder
                                .total_duration()
                                .map(|d| d.as_secs_f64())
                                .unwrap_or(0.0);
                            if stream.is_none() {
                                stream = Some(OutputStreamBuilder::open_default_stream().map_err(
                                    |e| format!("No se pudo abrir la salida de audio: {e}"),
                                )?);
                            }
                            if let Some(old) = sink.take() {
                                old.stop();
                            }
                            let next = Sink::connect_new(stream.as_ref().unwrap().mixer());
                            next.set_volume(state.volume);
                            next.append(decoder);
                            state.file_id = Some(id);
                            state.duration = duration;
                            sink = Some(next);
                            Ok(())
                        })();
                        let _ = reply.send(result);
                    }
                    Message::Control(control, reply) => {
                        let result = if let Some(sink) = sink.as_ref() {
                            match control {
                                Control::Pause => {
                                    sink.pause();
                                    Ok(())
                                }
                                Control::Resume => {
                                    sink.play();
                                    Ok(())
                                }
                                Control::Stop => {
                                    sink.stop();
                                    state.file_id = None;
                                    Ok(())
                                }
                                Control::Seek(seconds) => {
                                    if !seconds.is_finite() || seconds < 0.0 {
                                        Err("Posición no válida".into())
                                    } else {
                                        sink.try_seek(Duration::from_secs_f64(
                                            seconds.min(state.duration.max(0.0)),
                                        ))
                                        .map_err(|e| e.to_string())
                                    }
                                }
                                Control::Volume(volume) => {
                                    if volume.is_finite() && (0.0..=1.0).contains(&volume) {
                                        state.volume = volume;
                                        sink.set_volume(volume);
                                        Ok(())
                                    } else {
                                        Err("Volumen no válido".into())
                                    }
                                }
                            }
                        } else if let Control::Volume(volume) = control {
                            if volume.is_finite() && (0.0..=1.0).contains(&volume) {
                                state.volume = volume;
                                Ok(())
                            } else {
                                Err("Volumen no válido".into())
                            }
                        } else {
                            Err("Selecciona un audio primero".into())
                        };
                        let _ = reply.send(result);
                    }
                    Message::State(reply) => {
                        if let Some(sink) = sink.as_ref() {
                            state.position = sink.get_pos().as_secs_f64();
                            state.playing = !sink.is_paused() && !sink.empty();
                        }
                        let _ = reply.send(state.clone());
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
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|e| e.to_string())?
    }
    pub fn control(&self, control: Control) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Message::Control(control, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|e| e.to_string())?
    }
    pub fn state(&self) -> Result<PlayerState, String> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Message::State(tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())
    }
}
