use crate::backends::espeak::EspeakBackend;
use crate::backends::piper::PiperBackend;
use crate::backends::SpeechBackend;
use crate::backends::Voice;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender as MpscSender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::oneshot;

enum AudioMessage {
    PlayData {
        data: Vec<u8>,
        _voice: Option<String>,
        complete: Option<oneshot::Sender<()>>,
        channel: Option<String>,
    },
    ListVoices(oneshot::Sender<Vec<Voice>>),
    ListDownloadableVoices(oneshot::Sender<Vec<Voice>>),
    DownloadVoice(String, oneshot::Sender<std::io::Result<()>>),
    PlayAudio(String, oneshot::Sender<Result<(), String>>),
    StopAudio(oneshot::Sender<bool>),
    SetVolume(f32, oneshot::Sender<bool>),
    GetPlaybackStatus(oneshot::Sender<(bool, String)>),
    PlayAudioChannel(String, String, oneshot::Sender<Result<(), String>>),
    PlayFetchedAudio {
        data: Vec<u8>,
        url: String,
        resp_tx: oneshot::Sender<Result<(), String>>,
    },
    PlayFetchedAudioChannel {
        data: Vec<u8>,
        url: String,
        channel: String,
        resp_tx: oneshot::Sender<Result<(), String>>,
    },
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait AudioOutput: Send + Sync {
    fn speak(&self, text: &str, voice: Option<String>);
    async fn speak_blocking(&self, text: &str, voice: Option<String>);
    async fn list_voices(&self) -> Vec<Voice>;
    async fn list_downloadable_voices(&self) -> Vec<Voice>;
    async fn download_voice(&self, voice_id: String) -> std::io::Result<()>;
    async fn play_audio(&self, url: &str) -> Result<(), String>;
    async fn stop_audio(&self) -> bool;
    async fn set_volume(&self, volume: f32) -> bool;
    async fn get_playback_status(&self) -> (bool, String);
    fn speak_channel(&self, text: &str, voice: Option<String>, channel: &str);
    async fn play_audio_channel(&self, url: &str, channel: &str) -> Result<(), String>;
}

#[derive(Clone)]
pub struct AudioEngine {
    tx: MpscSender<AudioMessage>,
    backends: Arc<HashMap<String, Arc<dyn SpeechBackend>>>,
    _total_audio_buffer_size: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl AudioOutput for AudioEngine {
    fn speak(&self, text: &str, voice: Option<String>) {
        self.speak(text, voice)
    }
    async fn speak_blocking(&self, text: &str, voice: Option<String>) {
        self.speak_blocking(text, voice).await
    }
    async fn list_voices(&self) -> Vec<Voice> {
        self.list_voices().await
    }
    async fn list_downloadable_voices(&self) -> Vec<Voice> {
        self.list_downloadable_voices().await
    }
    async fn download_voice(&self, voice_id: String) -> std::io::Result<()> {
        self.download_voice(voice_id).await
    }
    async fn play_audio(&self, url: &str) -> Result<(), String> {
        self.play_audio(url).await
    }
    async fn stop_audio(&self) -> bool {
        self.stop_audio().await
    }
    async fn set_volume(&self, volume: f32) -> bool {
        self.set_volume(volume).await
    }
    async fn get_playback_status(&self) -> (bool, String) {
        self.get_playback_status().await
    }
    fn speak_channel(&self, text: &str, voice: Option<String>, channel: &str) {
        self.speak_channel(text, voice, channel)
    }
    async fn play_audio_channel(&self, url: &str, channel: &str) -> Result<(), String> {
        self.play_audio_channel(url, channel).await
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<AudioMessage>();

        let mut backends_map: HashMap<String, Arc<dyn SpeechBackend>> = HashMap::new();
        backends_map.insert("espeak".to_string(), Arc::new(EspeakBackend::new()));
        backends_map.insert("piper".to_string(), Arc::new(PiperBackend::new()));
        let backends = Arc::new(backends_map);
        let total_audio_buffer_size = Arc::new(AtomicUsize::new(0));

        let total_size_worker_clone = total_audio_buffer_size.clone();
        let backends_worker_clone = backends.clone();
        let internal_tx = tx.clone();

        thread::spawn(move || {
            let thread_tx = internal_tx;
            let audio_resource = OutputStream::try_default();
            let _stream_ownership;
            let stream_handle: Option<OutputStreamHandle> = match audio_resource {
                Ok((s, h)) => {
                    _stream_ownership = Some(s);
                    Some(h)
                }
                Err(e) => {
                    eprintln!(
                        "Audio Thread: No audio output device: {}. Headless mode.",
                        e
                    );
                    _stream_ownership = None;
                    None
                }
            };

            let mut current_volume: f32 = {
                let s = crate::config_loader::SETTINGS.read().unwrap();
                s.playback_volume
            };
            let mut current_url: Option<String> = None;
            let mut active_sinks: Vec<(Sink, Option<oneshot::Sender<()>>, usize)> = Vec::new();

            loop {
                let msg_result = rx.recv_timeout(Duration::from_millis(50));

                // Cleanup finished sinks
                let mut i = 0;
                while i < active_sinks.len() {
                    if active_sinks[i].0.empty() {
                        let (_, complete_tx, size) = active_sinks.remove(i);
                        if let Some(tx) = complete_tx {
                            let _ = tx.send(());
                        }
                        total_size_worker_clone.fetch_sub(size, Ordering::SeqCst);
                    } else {
                        i += 1;
                    }
                }

                let msg = match msg_result {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                };

                match msg {
                    AudioMessage::PlayData {
                        data,
                        _voice: _,
                        complete,
                        channel,
                    } => {
                        let data_len = data.len();
                        if let Some(ref handle) = stream_handle {
                            total_size_worker_clone.fetch_add(data_len, Ordering::SeqCst);
                            let cursor = Cursor::new(data);
                            match Sink::try_new(handle) {
                                Ok(sink) => match Decoder::new(cursor) {
                                    Ok(source) => {
                                        use rodio::source::ChannelVolume;
                                        use rodio::Source;
                                        if let Some(chan) = channel {
                                            let channel_vols = match chan.to_lowercase().as_str() {
                                                "left" => vec![current_volume, 0.0],
                                                "right" => vec![0.0, current_volume],
                                                "center" => {
                                                    vec![current_volume * 0.7, current_volume * 0.7]
                                                }
                                                _ => vec![current_volume, current_volume],
                                            };
                                            sink.append(ChannelVolume::new(
                                                source.convert_samples::<f32>(),
                                                channel_vols,
                                            ));
                                        } else {
                                            sink.set_volume(current_volume);
                                            sink.append(source.convert_samples::<f32>());
                                        }
                                        active_sinks.push((sink, complete, data_len));
                                    }
                                    Err(_) => {
                                        total_size_worker_clone
                                            .fetch_sub(data_len, Ordering::SeqCst);
                                        if let Some(tx) = complete {
                                            let _ = tx.send(());
                                        }
                                    }
                                },
                                Err(_) => {
                                    total_size_worker_clone.fetch_sub(data_len, Ordering::SeqCst);
                                    if let Some(tx) = complete {
                                        let _ = tx.send(());
                                    }
                                }
                            }
                        } else if let Some(tx) = complete {
                            let _ = tx.send(());
                        }
                    }
                    AudioMessage::ListVoices(resp_tx) => {
                        let mut all = Vec::new();
                        for (id, backend) in backends_worker_clone.iter() {
                            if let Ok(voices) = backend.list_voices() {
                                for mut v in voices {
                                    v.id = format!("{}:{}", id, v.id);
                                    all.push(v);
                                }
                            }
                        }
                        let _ = resp_tx.send(all);
                    }
                    AudioMessage::ListDownloadableVoices(resp_tx) => {
                        let mut all = Vec::new();
                        for (id, backend) in backends_worker_clone.iter() {
                            if let Ok(voices) = backend.list_downloadable_voices() {
                                for mut v in voices {
                                    v.id = format!("{}:{}", id, v.id);
                                    all.push(v);
                                }
                            }
                        }
                        let _ = resp_tx.send(all);
                    }
                    AudioMessage::DownloadVoice(full_id, resp_tx) => {
                        let (target, voice) = if full_id.contains(':') {
                            let parts: Vec<&str> = full_id.splitn(2, ':').collect();
                            (parts[0], parts[1])
                        } else {
                            ("piper", full_id.as_str())
                        };
                        if let Some(backend) = backends_worker_clone.get(target) {
                            let _ = resp_tx.send(backend.download_voice(voice));
                        } else {
                            let _ = resp_tx.send(Err(std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                "Backend not found",
                            )));
                        }
                    }
                    AudioMessage::PlayAudio(url, resp_tx) => {
                        let tx_clone = thread_tx.clone();
                        let total_size_counter = total_size_worker_clone.clone();
                        tokio::spawn(async move {
                            let (max_size_mb, global_max_size_mb, timeout_secs) = {
                                let s = crate::config_loader::SETTINGS.read().unwrap();
                                (
                                    s.max_audio_size_mb,
                                    s.global_audio_buffer_limit_mb,
                                    s.playback_timeout_secs,
                                )
                            };
                            let max_size_bytes = max_size_mb * 1024 * 1024;
                            let global_max_bytes = global_max_size_mb * 1024 * 1024;
                            let client = match reqwest::Client::builder()
                                .timeout(Duration::from_secs(timeout_secs))
                                .build()
                            {
                                Ok(c) => c,
                                Err(e) => {
                                    let _ = resp_tx.send(Err(format!("Client error: {}", e)));
                                    return;
                                }
                            };
                            match client.get(&url).send().await {
                                Ok(mut resp) => {
                                    if !resp.status().is_success() {
                                        let _ =
                                            resp_tx.send(Err(format!("HTTP {}", resp.status())));
                                        return;
                                    }
                                    if let Some(len) = resp.content_length() {
                                        if len > max_size_bytes {
                                            let _ = resp_tx.send(Err("Too large".into()));
                                            return;
                                        }
                                        if (total_size_counter.load(Ordering::SeqCst) as u64) + len
                                            > global_max_bytes
                                        {
                                            let _ = resp_tx.send(Err("Global limit".into()));
                                            return;
                                        }
                                    }
                                    let mut data = Vec::new();
                                    while let Ok(Some(chunk)) = resp.chunk().await {
                                        if (data.len() + chunk.len()) as u64 > max_size_bytes {
                                            let _ = resp_tx.send(Err("Too large".into()));
                                            return;
                                        }
                                        if (total_size_counter.load(Ordering::SeqCst) as u64)
                                            + (data.len() + chunk.len()) as u64
                                            > global_max_bytes
                                        {
                                            let _ = resp_tx.send(Err("Global limit".into()));
                                            return;
                                        }
                                        data.extend_from_slice(&chunk);
                                    }
                                    let _ = tx_clone.send(AudioMessage::PlayFetchedAudio {
                                        data,
                                        url,
                                        resp_tx,
                                    });
                                }
                                Err(e) => {
                                    let _ = resp_tx.send(Err(format!("Fetch error: {}", e)));
                                }
                            }
                        });
                    }
                    AudioMessage::PlayFetchedAudio { data, url, resp_tx } => {
                        if let Some(ref handle) = stream_handle {
                            let data_len = data.len();
                            total_size_worker_clone.fetch_add(data_len, Ordering::SeqCst);
                            let cursor = Cursor::new(data);
                            match Sink::try_new(handle) {
                                Ok(sink) => match Decoder::new(cursor) {
                                    Ok(source) => {
                                        use rodio::Source;
                                        sink.set_volume(current_volume);
                                        current_url = Some(url);
                                        sink.append(source.convert_samples::<f32>());
                                        active_sinks.push((sink, None, data_len));
                                        let _ = resp_tx.send(Ok(()));
                                    }
                                    Err(e) => {
                                        total_size_worker_clone
                                            .fetch_sub(data_len, Ordering::SeqCst);
                                        let _ = resp_tx.send(Err(format!("Decode error: {}", e)));
                                    }
                                },
                                Err(e) => {
                                    total_size_worker_clone.fetch_sub(data_len, Ordering::SeqCst);
                                    let _ = resp_tx.send(Err(format!("Sink error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp_tx.send(Ok(()));
                        }
                    }
                    AudioMessage::StopAudio(resp_tx) => {
                        let stopped = !active_sinks.is_empty();
                        for (_sink, tx, size) in active_sinks.drain(..) {
                            if let Some(tx) = tx {
                                let _ = tx.send(());
                            }
                            total_size_worker_clone.fetch_sub(size, Ordering::SeqCst);
                        }
                        current_url = None;
                        let _ = resp_tx.send(stopped);
                    }
                    AudioMessage::SetVolume(volume, resp_tx) => {
                        current_volume = volume.clamp(0.0, 1.0);
                        for (sink, _, _) in &active_sinks {
                            sink.set_volume(current_volume);
                        }
                        let _ = resp_tx.send(true);
                    }
                    AudioMessage::GetPlaybackStatus(resp_tx) => {
                        let _ = resp_tx.send((
                            !active_sinks.is_empty(),
                            current_url.clone().unwrap_or_default(),
                        ));
                    }
                    AudioMessage::PlayAudioChannel(url, channel, resp_tx) => {
                        let tx_clone = thread_tx.clone();
                        let total_size_counter = total_size_worker_clone.clone();
                        tokio::spawn(async move {
                            let (max_size_mb, global_max_size_mb, timeout_secs) = {
                                let s = crate::config_loader::SETTINGS.read().unwrap();
                                (
                                    s.max_audio_size_mb,
                                    s.global_audio_buffer_limit_mb,
                                    s.playback_timeout_secs,
                                )
                            };
                            let max_size_bytes = max_size_mb * 1024 * 1024;
                            let global_max_bytes = global_max_size_mb * 1024 * 1024;
                            let client = match reqwest::Client::builder()
                                .timeout(Duration::from_secs(timeout_secs))
                                .build()
                            {
                                Ok(c) => c,
                                Err(e) => {
                                    let _ = resp_tx.send(Err(format!("Client error: {}", e)));
                                    return;
                                }
                            };
                            match client.get(&url).send().await {
                                Ok(mut resp) => {
                                    if !resp.status().is_success() {
                                        let _ =
                                            resp_tx.send(Err(format!("HTTP {}", resp.status())));
                                        return;
                                    }
                                    if let Some(len) = resp.content_length() {
                                        if len > max_size_bytes {
                                            let _ = resp_tx.send(Err("Too large".into()));
                                            return;
                                        }
                                        if (total_size_counter.load(Ordering::SeqCst) as u64) + len
                                            > global_max_bytes
                                        {
                                            let _ = resp_tx.send(Err("Global limit".into()));
                                            return;
                                        }
                                    }
                                    let mut data = Vec::new();
                                    while let Ok(Some(chunk)) = resp.chunk().await {
                                        if (data.len() + chunk.len()) as u64 > max_size_bytes {
                                            let _ = resp_tx.send(Err("Too large".into()));
                                            return;
                                        }
                                        if (total_size_counter.load(Ordering::SeqCst) as u64)
                                            + (data.len() + chunk.len()) as u64
                                            > global_max_bytes
                                        {
                                            let _ = resp_tx.send(Err("Global limit".into()));
                                            return;
                                        }
                                        data.extend_from_slice(&chunk);
                                    }
                                    let _ = tx_clone.send(AudioMessage::PlayFetchedAudioChannel {
                                        data,
                                        url,
                                        channel,
                                        resp_tx,
                                    });
                                }
                                Err(e) => {
                                    let _ = resp_tx.send(Err(format!("Fetch error: {}", e)));
                                }
                            }
                        });
                    }
                    AudioMessage::PlayFetchedAudioChannel {
                        data,
                        url: _,
                        channel,
                        resp_tx,
                    } => {
                        if let Some(ref handle) = stream_handle {
                            let data_len = data.len();
                            total_size_worker_clone.fetch_add(data_len, Ordering::SeqCst);
                            let cursor = Cursor::new(data);
                            match Sink::try_new(handle) {
                                Ok(sink) => match Decoder::new(cursor) {
                                    Ok(source) => {
                                        use rodio::source::ChannelVolume;
                                        use rodio::Source;
                                        let vols = match channel.to_lowercase().as_str() {
                                            "left" => vec![current_volume, 0.0],
                                            "right" => vec![0.0, current_volume],
                                            "center" => {
                                                vec![current_volume * 0.7, current_volume * 0.7]
                                            }
                                            _ => vec![current_volume, current_volume],
                                        };
                                        sink.append(ChannelVolume::new(
                                            source.convert_samples::<f32>(),
                                            vols,
                                        ));
                                        active_sinks.push((sink, None, data_len));
                                        let _ = resp_tx.send(Ok(()));
                                    }
                                    Err(e) => {
                                        total_size_worker_clone
                                            .fetch_sub(data_len, Ordering::SeqCst);
                                        let _ = resp_tx.send(Err(format!("Decode error: {}", e)));
                                    }
                                },
                                Err(e) => {
                                    total_size_worker_clone.fetch_sub(data_len, Ordering::SeqCst);
                                    let _ = resp_tx.send(Err(format!("Sink error: {}", e)));
                                }
                            }
                        } else {
                            let _ = resp_tx.send(Ok(()));
                        }
                    }
                }
            }
        });

        Self {
            tx,
            backends,
            _total_audio_buffer_size: total_audio_buffer_size,
        }
    }

    pub fn speak(&self, text: &str, voice: Option<String>) {
        let backends = self.backends.clone();
        let tx = self.tx.clone();
        let text = text.to_string();
        tokio::spawn(async move {
            let (piper, default) = {
                let s = crate::config_loader::SETTINGS.read().unwrap();
                (s.piper_model.clone(), s.tts_backend.clone())
            };
            let (target, actual_v) = if let Some(ref v) = voice {
                if v.starts_with("piper:") {
                    ("piper".to_string(), Some(v[6..].to_string()))
                } else if v.starts_with("espeak:") {
                    ("espeak".to_string(), Some(v[7..].to_string()))
                } else {
                    (default, Some(v.clone()))
                }
            } else {
                (default, None)
            };
            if let Some(backend) = backends.get(&target) {
                let v_id = actual_v.or_else(|| if target == "piper" { Some(piper) } else { None });
                let res = tokio::task::spawn_blocking({
                    let backend = backend.clone();
                    let t = text.clone();
                    let v = v_id.clone();
                    move || backend.synthesize(&t, v.as_deref())
                })
                .await;
                if let Ok(Ok(data)) = res {
                    let _ = tx.send(AudioMessage::PlayData {
                        data,
                        _voice: voice,
                        complete: None,
                        channel: None,
                    });
                }
            }
        });
    }

    pub async fn speak_blocking(&self, text: &str, voice: Option<String>) {
        let (complete_tx, complete_rx) = oneshot::channel();
        let backends = self.backends.clone();
        let tx = self.tx.clone();
        let text = text.to_string();
        tokio::spawn(async move {
            let (piper, default) = {
                let s = crate::config_loader::SETTINGS.read().unwrap();
                (s.piper_model.clone(), s.tts_backend.clone())
            };
            let (target, actual_v) = if let Some(ref v) = voice {
                if v.starts_with("piper:") {
                    ("piper".to_string(), Some(v[6..].to_string()))
                } else if v.starts_with("espeak:") {
                    ("espeak".to_string(), Some(v[7..].to_string()))
                } else {
                    (default, Some(v.clone()))
                }
            } else {
                (default, None)
            };
            if let Some(backend) = backends.get(&target) {
                let v_id = actual_v.or_else(|| if target == "piper" { Some(piper) } else { None });
                let res = tokio::task::spawn_blocking({
                    let backend = backend.clone();
                    let t = text.clone();
                    let v = v_id.clone();
                    move || backend.synthesize(&t, v.as_deref())
                })
                .await;
                if let Ok(Ok(data)) = res {
                    let _ = tx.send(AudioMessage::PlayData {
                        data,
                        _voice: voice,
                        complete: Some(complete_tx),
                        channel: None,
                    });
                } else {
                    let _ = complete_tx.send(());
                }
            } else {
                let _ = complete_tx.send(());
            }
        });
        let _ = complete_rx.await;
    }

    pub async fn list_voices(&self) -> Vec<Voice> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::ListVoices(tx));
        rx.await.unwrap_or_default()
    }

    pub async fn list_downloadable_voices(&self) -> Vec<Voice> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::ListDownloadableVoices(tx));
        rx.await.unwrap_or_default()
    }

    pub async fn download_voice(&self, voice_id: String) -> std::io::Result<()> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::DownloadVoice(voice_id, tx));
        rx.await.map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Audio thread crashed")
        })?
    }

    pub async fn play_audio(&self, url: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::PlayAudio(url.to_string(), tx));
        rx.await.map_err(|_| "Audio thread crashed".to_string())?
    }

    pub async fn stop_audio(&self) -> bool {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::StopAudio(tx));
        rx.await.unwrap_or(false)
    }

    pub async fn set_volume(&self, volume: f32) -> bool {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::SetVolume(volume, tx));
        rx.await.unwrap_or(false)
    }

    pub async fn get_playback_status(&self) -> (bool, String) {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::GetPlaybackStatus(tx));
        rx.await.unwrap_or((false, String::new()))
    }

    pub fn speak_channel(&self, text: &str, voice: Option<String>, channel: &str) {
        let backends = self.backends.clone();
        let tx = self.tx.clone();
        let text = text.to_string();
        let channel = channel.to_string();
        tokio::spawn(async move {
            let (piper, default) = {
                let s = crate::config_loader::SETTINGS.read().unwrap();
                (s.piper_model.clone(), s.tts_backend.clone())
            };
            let (target, actual_v) = if let Some(ref v) = voice {
                if v.starts_with("piper:") {
                    ("piper".to_string(), Some(v[6..].to_string()))
                } else if v.starts_with("espeak:") {
                    ("espeak".to_string(), Some(v[7..].to_string()))
                } else {
                    (default, Some(v.clone()))
                }
            } else {
                (default, None)
            };
            if let Some(backend) = backends.get(&target) {
                let v_id = actual_v.or_else(|| if target == "piper" { Some(piper) } else { None });
                let res = tokio::task::spawn_blocking({
                    let backend = backend.clone();
                    let t = text.clone();
                    let v = v_id.clone();
                    move || backend.synthesize(&t, v.as_deref())
                })
                .await;
                if let Ok(Ok(data)) = res {
                    let _ = tx.send(AudioMessage::PlayData {
                        data,
                        _voice: voice,
                        complete: None,
                        channel: Some(channel),
                    });
                }
            }
        });
    }

    pub async fn play_audio_channel(&self, url: &str, channel: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::PlayAudioChannel(
            url.to_string(),
            channel.to_string(),
            tx,
        ));
        rx.await.map_err(|_| "Audio thread crashed".to_string())?
    }
}
