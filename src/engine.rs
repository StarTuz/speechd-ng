use crate::backends::SpeechBackend;
use crate::backends::espeak::EspeakBackend;
use crate::backends::piper::PiperBackend;
use crate::backends::Voice;
use std::io::Cursor;
use rodio::{Decoder, OutputStream, Sink};
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender as MpscSender};
use tokio::sync::oneshot;

enum AudioMessage {
    Speak(String, Option<String>, Option<oneshot::Sender<()>>),
    ListVoices(oneshot::Sender<Vec<Voice>>),
    ListDownloadableVoices(oneshot::Sender<Vec<Voice>>),
    DownloadVoice(String, oneshot::Sender<std::io::Result<()>>),
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
            let (_stream, stream_handle) = OutputStream::try_default().expect("No audio output device found");
            
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
                                    let cursor = Cursor::new(audio_data);
                                    match Sink::try_new(&stream_handle) {
                                        Ok(sink) => {
                                            match Decoder::new(cursor) {
                                                Ok(source) => {
                                                    use rodio::Source;
                                                    println!("Audio Thread: Playing audio...");
                                                    sink.append(source.convert_samples::<f32>());
                                                    sink.sleep_until_end();
                                                    println!("Audio Thread: Playback complete");
                                                }
                                                Err(e) => eprintln!("Failed to decode: {}", e),
                                            }
                                        }
                                        Err(e) => eprintln!("Failed to create sink: {}", e),
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
                    }
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
}
