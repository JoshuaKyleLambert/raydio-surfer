use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerStatus {
    Stopped,
    Connecting,
    Playing(String),
    Error(String),
}

pub enum AudioCommand {
    Play { name: String, url: String },
    Stop,
    SetVolume(f32),
}

#[derive(Clone)]
pub struct AudioController {
    sender: Sender<AudioCommand>,
    status: Arc<Mutex<PlayerStatus>>,
    volume: Arc<Mutex<f32>>,
}

impl AudioController {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let status = Arc::new(Mutex::new(PlayerStatus::Stopped));
        let volume = Arc::new(Mutex::new(0.5f32));

        let status_clone = Arc::clone(&status);
        let volume_clone = Arc::clone(&volume);

        thread::Builder::new()
            .name("audio-player".to_string())
            .spawn(move || {
                run_audio_worker(receiver, status_clone, volume_clone);
            })
            .expect("Failed to spawn audio worker thread");

        Self {
            sender,
            status,
            volume,
        }
    }

    pub fn play(&self, name: String, url: String) {
        let _ = self.sender.send(AudioCommand::Play { name, url });
    }

    pub fn stop(&self) {
        let _ = self.sender.send(AudioCommand::Stop);
    }

    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        if let Ok(mut v) = self.volume.lock() {
            *v = clamped;
        }
        let _ = self.sender.send(AudioCommand::SetVolume(clamped));
    }

    pub fn status(&self) -> PlayerStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(PlayerStatus::Stopped)
    }

    pub fn volume(&self) -> f32 {
        self.volume.lock().map(|v| *v).unwrap_or(0.5)
    }
}

/// A buffered, seekable stream reader for continuous internet radio streams
pub struct LiveStreamReader {
    inner: Arc<Mutex<StreamState>>,
    stop_signal: Arc<AtomicBool>,
}

struct StreamState {
    buffer: VecDeque<u8>,
    base_pos: u64,
    cursor_pos: u64,
    rx: Receiver<Vec<u8>>,
    stream_ended: bool,
}

impl LiveStreamReader {
    pub fn new(rx: Receiver<Vec<u8>>, stop_signal: Arc<AtomicBool>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamState {
                buffer: VecDeque::with_capacity(64 * 1024),
                base_pos: 0,
                cursor_pos: 0,
                rx,
                stream_ended: false,
            })),
            stop_signal,
        }
    }

    fn fetch_more(&self, state: &mut StreamState) {
        if state.stream_ended || self.stop_signal.load(Ordering::Relaxed) {
            return;
        }

        // Pull any immediately available chunks first
        let mut got_any = false;
        while let Ok(chunk) = state.rx.try_recv() {
            state.buffer.extend(chunk);
            got_any = true;
        }

        if !got_any {
            match state.rx.recv_timeout(Duration::from_millis(500)) {
                Ok(chunk) => {
                    state.buffer.extend(chunk);
                    while let Ok(additional) = state.rx.try_recv() {
                        state.buffer.extend(additional);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    state.stream_ended = true;
                }
            }
        }

        // Keep memory usage bounded while retaining a seeking window
        const MAX_KEEP_BEHIND: u64 = 128 * 1024; // 128 KB history
        let current_rel = state.cursor_pos.saturating_sub(state.base_pos);
        if current_rel > MAX_KEEP_BEHIND {
            let to_prune = (current_rel - MAX_KEEP_BEHIND) as usize;
            state.buffer.drain(..to_prune.min(state.buffer.len()));
            state.base_pos += to_prune as u64;
        }
    }
}

impl Read for LiveStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        loop {
            if self.stop_signal.load(Ordering::Relaxed) {
                return Ok(0);
            }

            let mut state = self.inner.lock().unwrap();

            let rel_pos = if state.cursor_pos >= state.base_pos {
                (state.cursor_pos - state.base_pos) as usize
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Read cursor before buffered stream range",
                ));
            };

            if rel_pos < state.buffer.len() {
                let available = state.buffer.len() - rel_pos;
                let to_copy = std::cmp::min(buf.len(), available);
                for (i, b) in state.buffer.iter().skip(rel_pos).take(to_copy).enumerate() {
                    buf[i] = *b;
                }
                state.cursor_pos += to_copy as u64;
                return Ok(to_copy);
            }

            if state.stream_ended {
                return Ok(0);
            }

            self.fetch_more(&mut state);
        }
    }
}

impl Seek for LiveStreamReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let mut state = self.inner.lock().unwrap();

        let target_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => {
                let current = state.cursor_pos as i64;
                let next = current + offset;
                if next < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Invalid seek to negative position",
                    ));
                }
                next as u64
            }
            SeekFrom::End(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Cannot seek from end on live stream",
                ));
            }
        };

        if target_pos < state.base_pos {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek target evicted from buffer",
            ));
        }

        while !state.stream_ended
            && target_pos >= state.base_pos + state.buffer.len() as u64
            && !self.stop_signal.load(Ordering::Relaxed)
        {
            self.fetch_more(&mut state);
        }

        state.cursor_pos = target_pos;
        Ok(target_pos)
    }
}

fn run_audio_worker(
    receiver: Receiver<AudioCommand>,
    status: Arc<Mutex<PlayerStatus>>,
    volume: Arc<Mutex<f32>>,
) {
    let device_sink: MixerDeviceSink = match DeviceSinkBuilder::open_default_sink() {
        Ok(sink) => sink,
        Err(e) => {
            if let Ok(mut s) = status.lock() {
                *s = PlayerStatus::Error(format!("Audio device error: {e}"));
            }
            return;
        }
    };

    let mut current_player: Option<Player> = None;
    let mut current_stop_signal: Option<Arc<AtomicBool>> = None;

    while let Ok(cmd) = receiver.recv() {
        match cmd {
            AudioCommand::Play { name, url } => {
                // Stop any current playback
                if let Some(stop) = current_stop_signal.take() {
                    stop.store(true, Ordering::Relaxed);
                }
                if let Some(player) = current_player.take() {
                    player.stop();
                }

                if let Ok(mut s) = status.lock() {
                    *s = PlayerStatus::Connecting;
                }

                let stop_signal = Arc::new(AtomicBool::new(false));
                let (chunk_tx, chunk_rx) = std::sync::mpsc::channel();

                let url_clone = url.clone();
                let stop_signal_clone = Arc::clone(&stop_signal);

                // Spawn network downloader thread
                thread::spawn(move || {
                    let runtime = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => rt,
                        Err(_) => return,
                    };

                    runtime.block_on(async move {
                        let client = reqwest::Client::builder()
                            .timeout(Duration::from_secs(12))
                            .build();

                        let client = match client {
                            Ok(c) => c,
                            Err(_) => return,
                        };

                        let resp = match client.get(&url_clone).send().await {
                            Ok(r) => r,
                            Err(_) => return,
                        };

                        use futures_util::StreamExt;
                        let mut stream = resp.bytes_stream();
                        while let Some(chunk_res) = stream.next().await {
                            if stop_signal_clone.load(Ordering::Relaxed) {
                                break;
                            }
                            match chunk_res {
                                Ok(bytes) => {
                                    if chunk_tx.send(bytes.to_vec()).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    });
                });

                let reader = LiveStreamReader::new(chunk_rx, Arc::clone(&stop_signal));

                match Decoder::new(reader) {
                    Ok(source) => {
                        let player = Player::connect_new(device_sink.mixer());
                        let current_vol = volume.lock().map(|v| *v).unwrap_or(0.5);
                        player.set_volume(current_vol);
                        player.append(source);

                        current_player = Some(player);
                        current_stop_signal = Some(stop_signal);

                        if let Ok(mut s) = status.lock() {
                            *s = PlayerStatus::Playing(name);
                        }
                    }
                    Err(e) => {
                        stop_signal.store(true, Ordering::Relaxed);
                        if let Ok(mut s) = status.lock() {
                            *s = PlayerStatus::Error(format!("Decode error: {e}"));
                        }
                    }
                }
            }
            AudioCommand::Stop => {
                if let Some(stop) = current_stop_signal.take() {
                    stop.store(true, Ordering::Relaxed);
                }
                if let Some(player) = current_player.take() {
                    player.stop();
                }
                if let Ok(mut s) = status.lock() {
                    *s = PlayerStatus::Stopped;
                }
            }
            AudioCommand::SetVolume(vol) => {
                if let Some(ref player) = current_player {
                    player.set_volume(vol);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom};
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::channel;

    #[test]
    fn test_live_stream_reader_read_and_seek() {
        let (tx, rx) = channel();
        let stop_signal = Arc::new(AtomicBool::new(false));

        tx.send(b"Hello ".to_vec()).unwrap();
        tx.send(b"World Radio!".to_vec()).unwrap();
        drop(tx); // Signal end of stream

        let mut reader = LiveStreamReader::new(rx, stop_signal);
        let mut buf = [0u8; 5];

        let bytes_read = reader.read(&mut buf).unwrap();
        assert_eq!(bytes_read, 5);
        assert_eq!(&buf, b"Hello");

        // Seek back to start
        let pos = reader.seek(SeekFrom::Start(0)).unwrap();
        assert_eq!(pos, 0);

        let mut full_buf = [0u8; 18];
        reader.read_exact(&mut full_buf).unwrap();
        assert_eq!(&full_buf, b"Hello World Radio!");
    }
}
