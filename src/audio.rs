use futures_util::StreamExt;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::collections::{HashSet, VecDeque};
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub mod playlist {
    use reqwest::Url;

    /// Resolve relative path or URL against a base URL
    pub fn resolve_url(base: &str, target: &str) -> String {
        let trimmed = target.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return trimmed.to_string();
        }
        if let Ok(base_url) = Url::parse(base)
            && let Ok(joined) = base_url.join(trimmed)
        {
            return joined.to_string();
        }
        trimmed.to_string()
    }

    /// Parse M3U / M3U8 playlist content and return extracted candidate stream URLs
    pub fn parse_m3u(content: &str, base_url: &str) -> Vec<String> {
        let mut urls = Vec::new();
        let mut lines = content.lines().map(|l| l.trim());
        while let Some(line) = lines.next() {
            if line.is_empty() {
                continue;
            }
            if line.starts_with('#') {
                // If it's HLS stream inf, the following line is the variant playlist URL
                if line.starts_with("#EXT-X-STREAM-INF:")
                    && let Some(next_line) = lines.next()
                {
                    let next_trimmed = next_line.trim();
                    if !next_trimmed.is_empty() && !next_trimmed.starts_with('#') {
                        urls.push(resolve_url(base_url, next_trimmed));
                    }
                }
                continue;
            }
            urls.push(resolve_url(base_url, line));
        }
        urls
    }

    /// Parse PLS playlist content and return extracted stream URLs
    pub fn parse_pls(content: &str, base_url: &str) -> Vec<String> {
        let mut entries: Vec<(u32, String)> = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if let Some((key, val)) = trimmed.split_once('=') {
                let key_lower = key.trim().to_lowercase();
                if key_lower.starts_with("file") {
                    let num_str = key_lower.strip_prefix("file").unwrap_or("");
                    let idx = num_str.parse::<u32>().unwrap_or(0);
                    let val_trimmed = val.trim();
                    if !val_trimmed.is_empty() {
                        entries.push((idx, resolve_url(base_url, val_trimmed)));
                    }
                }
            }
        }
        entries.sort_by_key(|e| e.0);
        entries.into_iter().map(|e| e.1).collect()
    }

    /// Parse XSPF (XML Shareable Playlist Format) content
    pub fn parse_xspf(content: &str, base_url: &str) -> Vec<String> {
        let mut urls = Vec::new();
        let mut remaining = content;
        while let Some(start_tag) = remaining.find("<location>") {
            let after_tag = &remaining[start_tag + 10..];
            if let Some(end_tag) = after_tag.find("</location>") {
                let loc = &after_tag[..end_tag].trim();
                let decoded = decode_xml_entities(loc);
                if !decoded.is_empty() {
                    urls.push(resolve_url(base_url, &decoded));
                }
                remaining = &after_tag[end_tag + 11..];
            } else {
                break;
            }
        }
        urls
    }

    /// Parse ASX (Advanced Stream Redirector) content
    pub fn parse_asx(content: &str, base_url: &str) -> Vec<String> {
        let mut urls = Vec::new();
        let lower = content.to_lowercase();
        let mut pos = 0;
        while let Some(ref_idx) = lower[pos..].find("<ref ") {
            let start = pos + ref_idx;
            if let Some(end) = lower[start..].find('>') {
                let tag_str = &content[start..start + end];
                if let Some(href_idx) = tag_str.to_lowercase().find("href") {
                    let after_href = tag_str[href_idx + 4..].trim_start();
                    if let Some(stripped) = after_href.strip_prefix('=') {
                        let after_eq = stripped.trim_start();
                        if let Some(quote) = after_eq.chars().next()
                            && (quote == '"' || quote == '\'')
                            && let Some(close_quote) = after_eq[1..].find(quote)
                        {
                            let href = &after_eq[1..=close_quote];
                            let decoded = decode_xml_entities(href.trim());
                            if !decoded.is_empty() {
                                urls.push(resolve_url(base_url, &decoded));
                            }
                        }
                    }
                }
                pos = start + end + 1;
            } else {
                break;
            }
        }
        urls
    }

    fn decode_xml_entities(s: &str) -> String {
        s.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
    }

    /// Detect if a URL extension suggests a playlist file
    pub fn is_playlist_url(url: &str) -> bool {
        let url_path = url.split('?').next().unwrap_or(url).to_lowercase();
        url_path.ends_with(".m3u")
            || url_path.ends_with(".m3u8")
            || url_path.ends_with(".pls")
            || url_path.ends_with(".xspf")
            || url_path.ends_with(".asx")
            || url_path.ends_with(".wax")
    }

    /// Detect if Content-Type header indicates a playlist
    pub fn is_playlist_content_type(ct: &str) -> bool {
        let ct_lower = ct.to_lowercase();
        ct_lower.contains("audio/x-scpls")
            || ct_lower.contains("application/pls+xml")
            || ct_lower.contains("audio/x-mpegurl")
            || ct_lower.contains("application/x-mpegurl")
            || ct_lower.contains("application/vnd.apple.mpegurl")
            || ct_lower.contains("audio/mpegurl")
            || ct_lower.contains("application/xspf+xml")
            || ct_lower.contains("video/x-ms-asf")
            || ct_lower.contains("audio/x-ms-wax")
            || ct_lower.contains("text/uri-list")
    }

    /// Parse general playlist content into candidate URLs
    pub fn parse_playlist_content(content: &str, base_url: &str) -> Vec<String> {
        let trimmed = content.trim();
        let trimmed_lower = trimmed.to_lowercase();
        if trimmed_lower.starts_with("[playlist]") || trimmed_lower.contains("file1=") {
            parse_pls(content, base_url)
        } else if trimmed_lower.starts_with("#extm3u") || trimmed_lower.contains("#extinf") {
            parse_m3u(content, base_url)
        } else if trimmed_lower.contains("<playlist") && trimmed_lower.contains("<location>") {
            parse_xspf(content, base_url)
        } else if trimmed_lower.contains("<asx") || trimmed_lower.contains("<ref ") {
            parse_asx(content, base_url)
        } else {
            // Default fallback: parse line-by-line
            parse_m3u(content, base_url)
        }
    }

    /// Check if M3U8 content is an HLS media segment playlist
    pub fn is_hls_media_playlist(content: &str) -> bool {
        content.contains("#EXT-X-TARGETDURATION") || content.contains("#EXT-X-MEDIA-SEQUENCE")
    }

    /// Extract media segments and durations from an HLS media playlist
    pub fn parse_hls_segments(content: &str, base_url: &str) -> (Vec<String>, f32) {
        let mut segments = Vec::new();
        let mut target_duration = 5.0f32;

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(val) = trimmed.strip_prefix("#EXT-X-TARGETDURATION:") {
                if let Ok(d) = val.trim().parse::<f32>() {
                    target_duration = d;
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                segments.push(resolve_url(base_url, trimmed));
            }
        }

        (segments, target_duration)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerStatus {
    Stopped,
    Connecting,
    Playing(String),
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub struct StreamDiagnostics {
    pub last_url: String,
    pub last_http_status: Option<u16>,
    pub last_content_type: Option<String>,
    pub last_error: Option<String>,
    pub bytes_streamed: usize,
    pub hls_active: bool,
    pub resolved_stream_type: Option<String>,
}

pub fn build_error_message(
    decoder_err: &rodio::decoder::DecoderError,
    diag: &StreamDiagnostics,
    format_hint: Option<&str>,
) -> String {
    let hint_suffix = match format_hint {
        Some(hint) if !hint.trim().is_empty() && hint.trim() != "Audio Stream" => {
            format!(" [{}]", hint.trim())
        }
        _ => String::new(),
    };

    if diag.bytes_streamed == 0 {
        if let Some(ref net_err) = diag.last_error {
            return format!("ERROR: {net_err}{hint_suffix}");
        }
        if let Some(code) = diag.last_http_status
            && code != 200
        {
            return format!("ERROR: HTTP {code}{hint_suffix}");
        }
        if let Some(ref ct) = diag.last_content_type
            && ct.contains("text/html")
        {
            return format!("ERROR: Webpage received instead of audio stream{hint_suffix}");
        }
        return format!("ERROR: Stream connection closed without audio data{hint_suffix}");
    }

    // Bytes were streamed, but decoding failed
    if let Some(ref ct) = diag.last_content_type {
        let ct_lower = ct.to_lowercase();
        if ct_lower.contains("audio/x-ms-wma")
            || ct_lower.contains("audio/wma")
            || ct_lower.contains("video/x-ms-asf")
        {
            return format!("ERROR: Unsupported stream format (WMA/ASF){hint_suffix}");
        }
        if ct_lower.contains("text/html") {
            return format!("ERROR: Invalid audio stream (HTTP HTML page){hint_suffix}");
        }
    }

    let err_str = decoder_err.to_string();
    let err_lower = err_str.to_lowercase();
    let clean_reason = if err_lower.contains("recognized") || err_lower.contains("no supported format") {
        "Unrecognized audio format"
    } else if err_lower.contains("unsupported") {
        "Unsupported audio codec/profile"
    } else if err_lower.contains("decode") {
        "Audio stream decode failed"
    } else {
        err_str.as_str()
    };

    if let Some(ref ct) = diag.last_content_type
        && !ct.is_empty()
        && !ct.starts_with("application/octet-stream")
    {
        format!("ERROR: {clean_reason} ({ct}){hint_suffix}")
    } else {
        format!("ERROR: {clean_reason}{hint_suffix}")
    }
}

pub enum AudioCommand {
    Play {
        name: String,
        url: String,
        format_hint: Option<String>,
    },
    Stop,
    SetVolume(f32),
}

#[derive(Clone)]
pub struct AudioController {
    sender: Sender<AudioCommand>,
    status: Arc<Mutex<PlayerStatus>>,
    volume: Arc<Mutex<f32>>,
    dsp_settings: Arc<Mutex<crate::dsp::AudioDspSettings>>,
}

impl AudioController {
    pub fn new() -> Self {
        Self::with_dsp_settings(crate::dsp::AudioDspSettings::default())
    }

    pub fn with_dsp_settings(initial_dsp: crate::dsp::AudioDspSettings) -> Self {
        let (sender, receiver) = channel();
        let status = Arc::new(Mutex::new(PlayerStatus::Stopped));
        let volume = Arc::new(Mutex::new(0.5f32));
        let dsp_settings = Arc::new(Mutex::new(initial_dsp));

        let status_clone = Arc::clone(&status);
        let volume_clone = Arc::clone(&volume);
        let dsp_clone = Arc::clone(&dsp_settings);

        thread::Builder::new()
            .name("audio-player".to_string())
            .spawn(move || {
                run_audio_worker(receiver, status_clone, volume_clone, dsp_clone);
            })
            .expect("Failed to spawn audio worker thread");

        Self {
            sender,
            status,
            volume,
            dsp_settings,
        }
    }

    pub fn dsp_settings(&self) -> Arc<Mutex<crate::dsp::AudioDspSettings>> {
        Arc::clone(&self.dsp_settings)
    }

    pub fn set_eq_enabled(&self, enabled: bool) {
        if let Ok(mut dsp) = self.dsp_settings.lock() {
            dsp.eq_enabled = enabled;
        }
    }

    pub fn set_eq_bands(&self, bands: [f32; crate::dsp::NUM_EQ_BANDS]) {
        if let Ok(mut dsp) = self.dsp_settings.lock() {
            dsp.eq_bands = bands;
        }
    }

    pub fn set_eq_band(&self, idx: usize, gain_db: f32) {
        if idx < crate::dsp::NUM_EQ_BANDS {
            if let Ok(mut dsp) = self.dsp_settings.lock() {
                dsp.eq_bands[idx] =
                    gain_db.clamp(crate::dsp::EQ_MIN_GAIN_DB, crate::dsp::EQ_MAX_GAIN_DB);
            }
        }
    }

    pub fn set_balance(&self, balance: f32) {
        if let Ok(mut dsp) = self.dsp_settings.lock() {
            dsp.balance = balance.clamp(crate::dsp::BALANCE_MIN, crate::dsp::BALANCE_MAX);
        }
    }

    pub fn play(&self, name: String, url: String) {
        let _ = self.sender.send(AudioCommand::Play {
            name,
            url,
            format_hint: None,
        });
    }

    pub fn play_with_hint(&self, name: String, url: String, format_hint: Option<String>) {
        let _ = self.sender.send(AudioCommand::Play {
            name,
            url,
            format_hint,
        });
    }

    pub fn play_station(&self, st: &crate::api::CachedStation) {
        self.play_with_hint(
            st.name.clone(),
            st.url.clone(),
            Some(st.format_description()),
        );
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

    #[allow(dead_code)]
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
        const MAX_KEEP_BEHIND: u64 = 256 * 1024; // 256 KB history
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
    dsp_settings: Arc<Mutex<crate::dsp::AudioDspSettings>>,
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
            AudioCommand::Play {
                name,
                url,
                format_hint,
            } => {
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
                let diagnostics = Arc::new(Mutex::new(StreamDiagnostics::default()));

                let url_clone = url.clone();
                let stop_signal_clone = Arc::clone(&stop_signal);
                let diag_clone = Arc::clone(&diagnostics);

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
                            .connect_timeout(Duration::from_secs(10))
                            .user_agent("RaydioSurfer/1.0")
                            .build();

                        let client = match client {
                            Ok(c) => c,
                            Err(_) => return,
                        };

                        stream_network_audio(
                            client,
                            url_clone,
                            chunk_tx,
                            stop_signal_clone,
                            diag_clone,
                        )
                        .await;
                    });
                });

                let reader = LiveStreamReader::new(chunk_rx, Arc::clone(&stop_signal));

                match Decoder::new(reader) {
                    Ok(source) => {
                        let player = Player::connect_new(device_sink.mixer());
                        let current_vol = volume.lock().map(|v| *v).unwrap_or(0.5);
                        player.set_volume(current_vol);
                        let dsp_source = crate::dsp::AudioDspSource::new(source, Arc::clone(&dsp_settings));
                        player.append(dsp_source);

                        current_player = Some(player);
                        current_stop_signal = Some(stop_signal);

                        if let Ok(mut s) = status.lock() {
                            *s = PlayerStatus::Playing(name);
                        }
                    }
                    Err(e) => {
                        stop_signal.store(true, Ordering::Relaxed);
                        let diag_info = diagnostics.lock().map(|d| d.clone()).unwrap_or_default();
                        let error_msg = build_error_message(&e, &diag_info, format_hint.as_deref());
                        if let Ok(mut s) = status.lock() {
                            *s = PlayerStatus::Error(error_msg);
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

async fn stream_network_audio(
    client: reqwest::Client,
    initial_url: String,
    chunk_tx: Sender<Vec<u8>>,
    stop_signal: Arc<AtomicBool>,
    diagnostics: Arc<Mutex<StreamDiagnostics>>,
) {
    let mut urls_to_try = vec![initial_url];
    let mut visited: HashSet<String> = HashSet::new();

    while let Some(current_url) = urls_to_try.pop() {
        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        if !visited.insert(current_url.clone()) || visited.len() > 10 {
            continue;
        }

        if let Ok(mut diag) = diagnostics.lock() {
            diag.last_url = current_url.clone();
        }

        let resp = match client.get(&current_url).send().await {
            Ok(r) => r,
            Err(e) => {
                if let Ok(mut diag) = diagnostics.lock() {
                    let err_desc = if e.is_timeout() {
                        "Connection timed out (10s)".to_string()
                    } else if e.is_connect() {
                        "Connection failed / host unreachable".to_string()
                    } else {
                        format!("Network request failed: {e}")
                    };
                    diag.last_error = Some(err_desc);
                }
                continue;
            }
        };

        let status = resp.status();
        let status_code = status.as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if let Ok(mut diag) = diagnostics.lock() {
            diag.last_http_status = Some(status_code);
            if !content_type.is_empty() {
                diag.last_content_type = Some(content_type.clone());
            }
        }

        if !status.is_success() {
            if let Ok(mut diag) = diagnostics.lock() {
                let reason = status.canonical_reason().unwrap_or("Error");
                diag.last_error = Some(format!("HTTP {status_code} {reason}"));
            }
            continue;
        }

        let is_playlist = playlist::is_playlist_url(&current_url)
            || playlist::is_playlist_content_type(&content_type);

        if is_playlist {
            if let Ok(mut diag) = diagnostics.lock() {
                diag.resolved_stream_type = Some("Playlist".to_string());
            }
            if let Ok(text) = resp.text().await {
                if playlist::is_hls_media_playlist(&text) {
                    if let Ok(mut diag) = diagnostics.lock() {
                        diag.hls_active = true;
                        diag.resolved_stream_type = Some("HLS Playlist".to_string());
                    }
                    stream_hls_loop(
                        client.clone(),
                        current_url,
                        text,
                        &chunk_tx,
                        &stop_signal,
                        Arc::clone(&diagnostics),
                    )
                    .await;
                    return;
                } else {
                    let candidates = playlist::parse_playlist_content(&text, &current_url);
                    if candidates.is_empty() {
                        if let Ok(mut diag) = diagnostics.lock() {
                            diag.last_error =
                                Some("Playlist contained no playable stream URLs".to_string());
                        }
                    } else {
                        for cand in candidates.into_iter().rev() {
                            if !visited.contains(&cand) {
                                urls_to_try.push(cand);
                            }
                        }
                    }
                }
            } else if let Ok(mut diag) = diagnostics.lock() {
                diag.last_error = Some("Failed to read playlist text from server".to_string());
            }
            continue;
        }

        // Direct audio stream
        let mut stream = resp.bytes_stream();
        let mut first_chunk = true;

        while let Some(chunk_res) = stream.next().await {
            if stop_signal.load(Ordering::Relaxed) {
                break;
            }
            match chunk_res {
                Ok(bytes) => {
                    // Check if initial bytes of an untyped response look like a playlist
                    if first_chunk {
                        first_chunk = false;
                        if let Ok(text_peek) = std::str::from_utf8(&bytes) {
                            let trimmed_lower = text_peek.trim_start().to_lowercase();
                            if trimmed_lower.starts_with("#extm3u")
                                || trimmed_lower.starts_with("[playlist]")
                                || trimmed_lower.starts_with("<playlist")
                                || trimmed_lower.starts_with("<asx")
                            {
                                let candidates =
                                    playlist::parse_playlist_content(text_peek, &current_url);
                                for cand in candidates.into_iter().rev() {
                                    if !visited.contains(&cand) {
                                        urls_to_try.push(cand);
                                    }
                                }
                                break;
                            }
                        }
                    }

                    let len = bytes.len();
                    if chunk_tx.send(bytes.to_vec()).is_err() {
                        return; // Receiver dropped or stopped
                    }
                    if let Ok(mut diag) = diagnostics.lock() {
                        diag.bytes_streamed += len;
                    }
                }
                Err(e) => {
                    if let Ok(mut diag) = diagnostics.lock() {
                        diag.last_error = Some(format!("Stream read error: {e}"));
                    }
                    break;
                }
            }
        }

        // If we streamed audio data, we're done (or stream disconnected)
        if !first_chunk {
            return;
        }
    }
}

async fn stream_hls_loop(
    client: reqwest::Client,
    playlist_url: String,
    initial_content: String,
    chunk_tx: &Sender<Vec<u8>>,
    stop_signal: &Arc<AtomicBool>,
    diagnostics: Arc<Mutex<StreamDiagnostics>>,
) {
    let mut played_segments: HashSet<String> = HashSet::new();
    let mut current_content = initial_content;

    loop {
        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        let (segments, target_duration) =
            playlist::parse_hls_segments(&current_content, &playlist_url);

        if segments.is_empty()
            && let Ok(mut diag) = diagnostics.lock()
            && diag.bytes_streamed == 0
        {
            diag.last_error = Some("HLS manifest contained no media segments".to_string());
        }

        for seg_url in segments {
            if stop_signal.load(Ordering::Relaxed) {
                return;
            }
            if played_segments.insert(seg_url.clone()) {
                if let Ok(mut diag) = diagnostics.lock() {
                    diag.last_url = seg_url.clone();
                }
                if let Ok(seg_resp) = client.get(&seg_url).send().await {
                    if seg_resp.status().is_success() {
                        if let Ok(bytes) = seg_resp.bytes().await {
                            let len = bytes.len();
                            if chunk_tx.send(bytes.to_vec()).is_err() {
                                return;
                            }
                            if let Ok(mut diag) = diagnostics.lock() {
                                diag.bytes_streamed += len;
                            }
                        }
                    } else if let Ok(mut diag) = diagnostics.lock() {
                        let st = seg_resp.status();
                        diag.last_error = Some(format!(
                            "HLS segment HTTP {} {}",
                            st.as_u16(),
                            st.canonical_reason().unwrap_or("Error")
                        ));
                    }
                }
            }
        }

        // Limit tracking set size to prevent unbounded memory growth
        if played_segments.len() > 100 {
            played_segments.clear();
        }

        // Wait before refreshing the live playlist
        let sleep_duration = Duration::from_secs_f32((target_duration * 0.5).clamp(1.0, 10.0));
        tokio::time::sleep(sleep_duration).await;

        if stop_signal.load(Ordering::Relaxed) {
            break;
        }

        match client.get(&playlist_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    current_content = text;
                }
            }
            _ => break,
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

    #[test]
    fn test_playlist_m3u_parsing() {
        let m3u_data = r#"
#EXTM3U
#EXTINF:-1,Radio 1 Rock
http://stream1.example.com/rock.mp3
#EXTINF:-1,Radio 1 Backup
http://stream2.example.com/rock.mp3
"#;
        let urls = playlist::parse_m3u(m3u_data, "http://example.com/listen.m3u");
        assert_eq!(
            urls,
            vec![
                "http://stream1.example.com/rock.mp3".to_string(),
                "http://stream2.example.com/rock.mp3".to_string(),
            ]
        );
    }

    #[test]
    fn test_playlist_pls_parsing() {
        let pls_data = r#"
[playlist]
NumberOfEntries=2
File1=http://stream1.example.com:8000/live
Title1=Jazz Radio
Length1=-1
File2=http://stream2.example.com:8000/live
Title2=Jazz Radio (Backup)
Length2=-1
Version=2
"#;
        let urls = playlist::parse_pls(pls_data, "http://example.com/listen.pls");
        assert_eq!(
            urls,
            vec![
                "http://stream1.example.com:8000/live".to_string(),
                "http://stream2.example.com:8000/live".to_string(),
            ]
        );
    }

    #[test]
    fn test_playlist_xspf_parsing() {
        let xspf_data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<playlist version="1" xmlns="http://xspf.org/ns/0/">
    <trackList>
        <track>
            <location>http://stream.example.com/synthwave</location>
            <title>Synthwave Station</title>
        </track>
    </trackList>
</playlist>
"#;
        let urls = playlist::parse_xspf(xspf_data, "http://example.com/listen.xspf");
        assert_eq!(urls, vec!["http://stream.example.com/synthwave".to_string()]);
    }

    #[test]
    fn test_playlist_asx_parsing() {
        let asx_data = r#"
<asx version="3.0">
    <entry>
        <title>Ambient Station</title>
        <ref href="http://stream.example.com/ambient.mp3" />
    </entry>
</asx>
"#;
        let urls = playlist::parse_asx(asx_data, "http://example.com/listen.asx");
        assert_eq!(urls, vec!["http://stream.example.com/ambient.mp3".to_string()]);
    }

    #[test]
    fn test_hls_segment_parsing() {
        let hls_data = r#"
#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:6
#EXT-X-MEDIA-SEQUENCE:100
#EXTINF:6.0,
segment100.aac
#EXTINF:6.0,
segment101.aac
"#;
        let (segments, target_duration) =
            playlist::parse_hls_segments(hls_data, "http://example.com/hls/live.m3u8");
        assert_eq!(target_duration, 6.0);
        assert_eq!(
            segments,
            vec![
                "http://example.com/hls/segment100.aac".to_string(),
                "http://example.com/hls/segment101.aac".to_string(),
            ]
        );
    }

    #[test]
    fn test_build_error_message_formatting() {
        let err = rodio::decoder::DecoderError::UnrecognizedFormat;

        // 1. HTTP 404 with format hint
        let diag_404 = StreamDiagnostics {
            last_http_status: Some(404),
            last_error: Some("HTTP 404 Not Found".to_string()),
            bytes_streamed: 0,
            ..Default::default()
        };
        let msg_404 = build_error_message(&err, &diag_404, Some("MP3 128kbps"));
        assert_eq!(msg_404, "ERROR: HTTP 404 Not Found [MP3 128kbps]");

        // 2. Connection Timeout with HLS format hint
        let diag_timeout = StreamDiagnostics {
            last_error: Some("Connection timed out (10s)".to_string()),
            bytes_streamed: 0,
            ..Default::default()
        };
        let msg_timeout = build_error_message(&err, &diag_timeout, Some("HLS AAC 320kbps"));
        assert_eq!(
            msg_timeout,
            "ERROR: Connection timed out (10s) [HLS AAC 320kbps]"
        );

        // 3. Unsupported WMA codec
        let diag_wma = StreamDiagnostics {
            last_content_type: Some("audio/x-ms-wma".to_string()),
            bytes_streamed: 1024,
            ..Default::default()
        };
        let msg_wma = build_error_message(&err, &diag_wma, Some("WMA 64kbps"));
        assert_eq!(
            msg_wma,
            "ERROR: Unsupported stream format (WMA/ASF) [WMA 64kbps]"
        );

        // 4. Decode failed on streamed bytes with format
        let diag_decode = StreamDiagnostics {
            last_content_type: Some("audio/aac".to_string()),
            bytes_streamed: 4096,
            ..Default::default()
        };
        let msg_decode = build_error_message(&err, &diag_decode, Some("AAC 256kbps"));
        assert_eq!(
            msg_decode,
            "ERROR: Unrecognized audio format (audio/aac) [AAC 256kbps]"
        );
    }
}
