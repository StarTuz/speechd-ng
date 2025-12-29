use crate::backends::SpeechBackend;
use crate::backends::espeak::EspeakBackend;
use crate::backends::piper::PiperBackend;
use crate::backends::Voice;
use std::io::Cursor;
use rodio::{Decoder, OutputStream, Sink, OutputStreamHandle};
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender as MpscSender};
use tokio::sync::oneshot;

enum AudioMessage {
    Speak(String, Option<String>, Option<oneshot::Sender<()>>),
    ListVoices(oneshot::Sender<Vec<Voice>>),
    ListDownloadableVoices(oneshot::Sender<Vec<Voice>>),
    DownloadVoice(String, oneshot::Sender<std::io::Result<()>>),
    // Phase 15: Streaming Media Player
    PlayAudio(String, oneshot::Sender<Result<(), String>>),  // (url, response)
    StopAudio(oneshot::Sender<bool>),                        // Returns true if stopped
    SetVolume(f32, oneshot::Sender<bool>),                   // (volume 0.0-1.0, success)
    GetPlaybackStatus(oneshot::Sender<(bool, String)>),      // (is_playing, current_url)
    // Phase 16: Multi-Channel Audio
    SpeakChannel(String, Option<String>, String, Option<oneshot::Sender<()>>),  // (text, voice, channel, complete)
    PlayAudioChannel(String, String, oneshot::Sender<Result<(), String>>),      // (url, channel, response)
}

#[derive(Clone)]
pub struct AudioEngine {
    tx: MpscSender<AudioMessage>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<AudioMessage>();
        
        // Initialize multiple backends
        let mut backends: HashMap<String, Box<dyn SpeechBackend>> = HashMap::new();
        backends.insert("espeak".to_string(), Box::new(EspeakBackend::new()));
        backends.insert("piper".to_string(), Box::new(PiperBackend::new()));
        
        thread::spawn(move || {
            // Audio stream must live on this thread
            let audio_resource = OutputStream::try_default();
            
            let _stream_ownership;
            let stream_handle: Option<OutputStreamHandle> = match audio_resource {
                Ok((s, h)) => {
                    _stream_ownership = Some(s);
                    Some(h)
                }
                Err(e) => {
                    eprintln!("Audio Thread: No audio output device found: {}. Headless mode active.", e);
                    _stream_ownership = None;
                    None
                }
            };
            
            // Phase 15: Playback state tracking
            let mut current_volume: f32 = {
                let s = crate::config_loader::SETTINGS.read().unwrap();
                s.playback_volume
            };
            let mut current_url: Option<String> = None;
            let mut active_sink: Option<Sink> = None;
            
            while let Ok(msg) = rx.recv() {
                match msg {
                    AudioMessage::Speak(text, voice, complete_tx) => {
                        let (piper_model, default_backend) = {
                            let s = crate::config_loader::SETTINGS.read().unwrap();
                            (s.piper_model.clone(), s.tts_backend.clone())
                        };

                        // 1. Determine which backend to use based on the voice prefix or default
                        let (target_backend_id, actual_voice) = if let Some(ref v) = voice {
                            if v.starts_with("piper:") {
                                ("piper", Some(&v[6..]))
                            } else if v.starts_with("espeak:") {
                                ("espeak", Some(&v[7..]))
                            } else {
                                // No prefix, use the user's preferred default backend
                                (default_backend.as_str(), Some(v.as_str()))
                            }
                        } else {
                            // No voice specified at all, use default backend and its default voice
                            (default_backend.as_str(), None)
                        };

                        // 2. Locate the backend
                        if let Some(backend) = backends.get(target_backend_id) {
                            // Determine the base voice ID, stripping any prefixes if present in config or parameters
                            let raw_voice = if target_backend_id == "piper" && actual_voice.is_none() {
                                Some(piper_model.as_str())
                            } else {
                                actual_voice
                            };

                            let voice_id = raw_voice.map(|v| {
                                if v.starts_with("piper:") {
                                    &v[6..]
                                } else if v.starts_with("espeak:") {
                                    &v[7..]
                                } else {
                                    v
                                }
                            });

                            println!("Audio Thread: Routing '{}' to {} (voice: {:?})", text, target_backend_id, voice_id);

                            match backend.synthesize(&text, voice_id) {
                                Ok(audio_data) => {
                                    println!("Audio Thread: Received {} bytes of audio data", audio_data.len());
                                    if let Some(ref handle) = stream_handle {
                                        let cursor = Cursor::new(audio_data);
                                        match Sink::try_new(handle) {
                                            Ok(sink) => {
                                                match Decoder::new(cursor) {
                                                    Ok(source) => {
                                                        use rodio::Source;
                                                        println!("Audio Thread: Playing audio...");
                                                        sink.set_volume(current_volume);
                                                        sink.append(source.convert_samples::<f32>());
                                                        sink.sleep_until_end();
                                                        println!("Audio Thread: Playback complete");
                                                    }
                                                    Err(e) => eprintln!("Failed to decode: {}", e),
                                                }
                                            }
                                            Err(e) => eprintln!("Failed to create sink: {}", e),
                                        }
                                    } else {
                                        println!("Audio Thread (Headless): Skipping playback of synthesized audio.");
                                    }
                                }
                                Err(e) => eprintln!("Backend {} error: {}", target_backend_id, e),
                            }
                        } else {
                            eprintln!("Error: Unknown backend '{}'", target_backend_id);
                        }
                        if let Some(tx) = complete_tx {
                            let _ = tx.send(());
                        }
                    },
                    AudioMessage::ListVoices(resp_tx) => {
                        let mut all_voices = Vec::new();
                        for (id, backend) in &backends {
                            if let Ok(voices) = backend.list_voices() {
                                for mut v in voices {
                                    v.id = format!("{}:{}", id, v.id);
                                    all_voices.push(v);
                                }
                            }
                        }
                        let _ = resp_tx.send(all_voices);
                    },
                    AudioMessage::ListDownloadableVoices(resp_tx) => {
                        let mut all_voices = Vec::new();
                        for (id, backend) in &backends {
                            if let Ok(voices) = backend.list_downloadable_voices() {
                                for mut v in voices {
                                    v.id = format!("{}:{}", id, v.id);
                                    all_voices.push(v);
                                }
                            }
                        }
                        let _ = resp_tx.send(all_voices);
                    },
                    AudioMessage::DownloadVoice(full_id, resp_tx) => {
                        let (target_backend_id, voice_id) = if full_id.contains(':') {
                            let parts: Vec<&str> = full_id.splitn(2, ':').collect();
                            (parts[0], parts[1])
                        } else {
                            ("piper", full_id.as_str()) // Default to piper if no prefix
                        };

                        if let Some(backend) = backends.get(target_backend_id) {
                            println!("Audio Thread: Downloading voice '{}' for backend '{}'", voice_id, target_backend_id);
                            let res = backend.download_voice(voice_id);
                            let _ = resp_tx.send(res);
                        } else {
                            let _ = resp_tx.send(Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Backend {} not found", target_backend_id))));
                        }
                    },
                    
                    // ========== Phase 15: Streaming Media Player ==========
                    AudioMessage::PlayAudio(url, resp_tx) => {
                        println!("Audio Thread: PlayAudio request for URL: {}", url);
                        
                        // Validate URL
                        if !url.starts_with("http://") && !url.starts_with("https://") {
                            let _ = resp_tx.send(Err("Invalid URL: must start with http:// or https://".to_string()));
                            continue;
                        }
                        
                        // Get config
                        let (max_size_bytes, timeout_secs) = {
                            let s = crate::config_loader::SETTINGS.read().unwrap();
                            (s.max_audio_size_mb * 1024 * 1024, s.playback_timeout_secs)
                        };
                        
                        // Download the audio
                        let client = match reqwest::blocking::Client::builder()
                            .timeout(std::time::Duration::from_secs(timeout_secs))
                            .build() 
                        {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = resp_tx.send(Err(format!("Failed to create HTTP client: {}", e)));
                                continue;
                            }
                        };
                        
                        match client.get(&url).send() {
                            Ok(response) => {
                                if !response.status().is_success() {
                                    let _ = resp_tx.send(Err(format!("HTTP error: {}", response.status())));
                                    continue;
                                }
                                
                                // Check content length if available
                                if let Some(len) = response.content_length() {
                                    if len > max_size_bytes {
                                        let _ = resp_tx.send(Err(format!("Audio too large: {} bytes (max: {} bytes)", len, max_size_bytes)));
                                        continue;
                                    }
                                }
                                
                                match response.bytes() {
                                    Ok(audio_data) => {
                                        if audio_data.len() as u64 > max_size_bytes {
                                            let _ = resp_tx.send(Err(format!("Audio too large: {} bytes", audio_data.len())));
                                            continue;
                                        }
                                        
                                        println!("Audio Thread: Downloaded {} bytes from {}", audio_data.len(), url);
                                        
                                        if let Some(ref handle) = stream_handle {
                                            let cursor = Cursor::new(audio_data.to_vec());
                                            match Sink::try_new(handle) {
                                                Ok(sink) => {
                                                    match Decoder::new(cursor) {
                                                        Ok(source) => {
                                                            use rodio::Source;
                                                            sink.set_volume(current_volume);
                                                            current_url = Some(url.clone());
                                                            active_sink = Some(sink);
                                                            
                                                            // Get reference to active sink
                                                            if let Some(ref s) = active_sink {
                                                                println!("Audio Thread: Playing audio from URL...");
                                                                s.append(source.convert_samples::<f32>());
                                                                let _ = resp_tx.send(Ok(()));
                                                                s.sleep_until_end();
                                                                println!("Audio Thread: URL playback complete");
                                                            }
                                                            
                                                            // Clear state after playback
                                                            current_url = None;
                                                            active_sink = None;
                                                        }
                                                        Err(e) => {
                                                            let _ = resp_tx.send(Err(format!("Failed to decode audio: {}", e)));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = resp_tx.send(Err(format!("Failed to create audio sink: {}", e)));
                                                }
                                            }
                                        } else {
                                            println!("Audio Thread (Headless): Skipping URL playback");
                                            let _ = resp_tx.send(Ok(()));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(Err(format!("Failed to read audio data: {}", e)));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = resp_tx.send(Err(format!("Failed to fetch audio: {}", e)));
                            }
                        }
                    },
                    
                    AudioMessage::StopAudio(resp_tx) => {
                        if let Some(ref sink) = active_sink {
                            sink.stop();
                            current_url = None;
                            active_sink = None;
                            println!("Audio Thread: Stopped playback");
                            let _ = resp_tx.send(true);
                        } else {
                            let _ = resp_tx.send(false);
                        }
                    },
                    
                    AudioMessage::SetVolume(volume, resp_tx) => {
                        let clamped = volume.clamp(0.0, 1.0);
                        current_volume = clamped;
                        if let Some(ref sink) = active_sink {
                            sink.set_volume(clamped);
                        }
                        println!("Audio Thread: Volume set to {:.2}", clamped);
                        let _ = resp_tx.send(true);
                    },
                    
                    AudioMessage::GetPlaybackStatus(resp_tx) => {
                        let is_playing = active_sink.as_ref().map_or(false, |s| !s.empty());
                        let url = current_url.clone().unwrap_or_default();
                        let _ = resp_tx.send((is_playing, url));
                    },
                    
                    // ========== Phase 16: Multi-Channel Audio ==========
                    AudioMessage::SpeakChannel(text, voice, channel, complete_tx) => {
                        let (piper_model, default_backend) = {
                            let s = crate::config_loader::SETTINGS.read().unwrap();
                            (s.piper_model.clone(), s.tts_backend.clone())
                        };

                        // Determine backend and voice
                        let (target_backend_id, actual_voice) = if let Some(ref v) = voice {
                            if v.starts_with("piper:") {
                                ("piper", Some(&v[6..]))
                            } else if v.starts_with("espeak:") {
                                ("espeak", Some(&v[7..]))
                            } else {
                                (default_backend.as_str(), Some(v.as_str()))
                            }
                        } else {
                            (default_backend.as_str(), None)
                        };

                        if let Some(backend) = backends.get(target_backend_id) {
                            let raw_voice = if target_backend_id == "piper" && actual_voice.is_none() {
                                Some(piper_model.as_str())
                            } else {
                                actual_voice
                            };

                            let voice_id = raw_voice.map(|v| {
                                if v.starts_with("piper:") { &v[6..] }
                                else if v.starts_with("espeak:") { &v[7..] }
                                else { v }
                            });

                            println!("Audio Thread: SpeakChannel '{}' to {} (voice: {:?}, channel: {})", 
                                text, target_backend_id, voice_id, channel);

                            match backend.synthesize(&text, voice_id) {
                                Ok(audio_data) => {
                                    if let Some(ref handle) = stream_handle {
                                        let cursor = Cursor::new(audio_data);
                                        match Sink::try_new(handle) {
                                            Ok(sink) => {
                                                match Decoder::new(cursor) {
                                                    Ok(source) => {
                                                        use rodio::Source;
                                                        use rodio::source::ChannelVolume;
                                                        
                                                        // Get channel volumes for L/R stereo output
                                                        // ChannelVolume makes mono and routes to specified channels
                                                        let channel_vols: Vec<f32> = match channel.to_lowercase().as_str() {
                                                            "left" => vec![current_volume, 0.0],  // Left only
                                                            "right" => vec![0.0, current_volume], // Right only
                                                            "center" => vec![current_volume * 0.7, current_volume * 0.7], // Both
                                                            _ => vec![current_volume, current_volume], // Full stereo
                                                        };
                                                        
                                                        println!("Audio Thread: Playing to channel '{}' (L:{:.2}, R:{:.2})", 
                                                            channel, channel_vols[0], channel_vols[1]);
                                                        
                                                        // Wrap source with ChannelVolume for true L/R separation
                                                        let panned = ChannelVolume::new(
                                                            source.convert_samples::<f32>(),
                                                            channel_vols
                                                        );
                                                        
                                                        sink.append(panned);
                                                        sink.sleep_until_end();
                                                    }
                                                    Err(e) => eprintln!("Failed to decode: {}", e),
                                                }
                                            }
                                            Err(e) => eprintln!("Failed to create sink: {}", e),
                                        }
                                    }
                                }
                                Err(e) => eprintln!("Backend {} error: {}", target_backend_id, e),
                            }
                        }
                        if let Some(tx) = complete_tx {
                            let _ = tx.send(());
                        }
                    },
                    
                    AudioMessage::PlayAudioChannel(url, channel, resp_tx) => {
                        println!("Audio Thread: PlayAudioChannel {} to {}", url, channel);
                        
                        if !url.starts_with("http://") && !url.starts_with("https://") {
                            let _ = resp_tx.send(Err("Invalid URL: must start with http:// or https://".to_string()));
                            continue;
                        }
                        
                        let (max_size_bytes, timeout_secs) = {
                            let s = crate::config_loader::SETTINGS.read().unwrap();
                            (s.max_audio_size_mb * 1024 * 1024, s.playback_timeout_secs)
                        };
                        
                        let client = match reqwest::blocking::Client::builder()
                            .timeout(std::time::Duration::from_secs(timeout_secs))
                            .build()
                        {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = resp_tx.send(Err(format!("HTTP client error: {}", e)));
                                continue;
                            }
                        };
                        
                        match client.get(&url).send() {
                            Ok(response) => {
                                if !response.status().is_success() {
                                    let _ = resp_tx.send(Err(format!("HTTP error: {}", response.status())));
                                    continue;
                                }
                                
                                if let Some(len) = response.content_length() {
                                    if len > max_size_bytes {
                                        let _ = resp_tx.send(Err(format!("Audio too large: {} bytes", len)));
                                        continue;
                                    }
                                }
                                
                                match response.bytes() {
                                    Ok(audio_data) => {
                                        if let Some(ref handle) = stream_handle {
                                            let cursor = Cursor::new(audio_data.to_vec());
                                            match Sink::try_new(handle) {
                                                Ok(sink) => {
                                                    match Decoder::new(cursor) {
                                                        Ok(source) => {
                                                            use rodio::Source;
                                                            use rodio::source::ChannelVolume;
                                                            
                                                            // Get channel volumes for L/R stereo output
                                                            let channel_vols: Vec<f32> = match channel.to_lowercase().as_str() {
                                                                "left" => vec![current_volume, 0.0],
                                                                "right" => vec![0.0, current_volume],
                                                                "center" => vec![current_volume * 0.7, current_volume * 0.7],
                                                                _ => vec![current_volume, current_volume],
                                                            };
                                                            
                                                            println!("Audio Thread: Playing URL to channel '{}' (L:{:.2}, R:{:.2})", 
                                                                channel, channel_vols[0], channel_vols[1]);
                                                            
                                                            let panned = ChannelVolume::new(
                                                                source.convert_samples::<f32>(),
                                                                channel_vols
                                                            );
                                                            
                                                            sink.append(panned);
                                                            let _ = resp_tx.send(Ok(()));
                                                            sink.sleep_until_end();
                                                        }
                                                        Err(e) => {
                                                            let _ = resp_tx.send(Err(format!("Decode error: {}", e)));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = resp_tx.send(Err(format!("Sink error: {}", e)));
                                                }
                                            }
                                        } else {
                                            let _ = resp_tx.send(Ok(()));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(Err(format!("Read error: {}", e)));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = resp_tx.send(Err(format!("Fetch error: {}", e)));
                            }
                        }
                    },
                }
            }
        });

        Self { tx }
    }

    pub fn speak(&self, text: &str, voice: Option<String>) {
        let _ = self.tx.send(AudioMessage::Speak(text.to_string(), voice, None));
    }

    pub async fn speak_blocking(&self, text: &str, voice: Option<String>) {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::Speak(text.to_string(), voice, Some(tx)));
        let _ = rx.await;
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
        rx.await.map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Audio thread crashed"))?
    }
    
    // ========== Phase 15: Streaming Media Player ==========
    
    /// Play audio from a URL
    /// Returns Ok(()) on success, Err(message) on failure
    pub async fn play_audio(&self, url: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::PlayAudio(url.to_string(), tx));
        rx.await.map_err(|_| "Audio thread crashed".to_string())?
    }
    
    /// Stop current audio playback
    /// Returns true if something was stopped
    pub async fn stop_audio(&self) -> bool {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::StopAudio(tx));
        rx.await.unwrap_or(false)
    }
    
    /// Set playback volume (0.0 - 1.0)
    /// Returns true on success
    pub async fn set_volume(&self, volume: f32) -> bool {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::SetVolume(volume, tx));
        rx.await.unwrap_or(false)
    }
    
    /// Get current playback status
    /// Returns (is_playing, current_url)
    pub async fn get_playback_status(&self) -> (bool, String) {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::GetPlaybackStatus(tx));
        rx.await.unwrap_or((false, String::new()))
    }
    
    // ========== Phase 16: Multi-Channel Audio ==========
    
    /// Speak text to a specific channel (left, right, center, or stereo)
    pub fn speak_channel(&self, text: &str, voice: Option<String>, channel: &str) {
        let _ = self.tx.send(AudioMessage::SpeakChannel(
            text.to_string(), 
            voice, 
            channel.to_string(), 
            None
        ));
    }
    
    /// Speak text to a specific channel and wait for completion
    pub async fn speak_channel_blocking(&self, text: &str, voice: Option<String>, channel: &str) {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::SpeakChannel(
            text.to_string(), 
            voice, 
            channel.to_string(), 
            Some(tx)
        ));
        let _ = rx.await;
    }
    
    /// Play audio from URL to a specific channel
    pub async fn play_audio_channel(&self, url: &str, channel: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::PlayAudioChannel(
            url.to_string(), 
            channel.to_string(), 
            tx
        ));
        rx.await.map_err(|_| "Audio thread crashed".to_string())?
    }
}
