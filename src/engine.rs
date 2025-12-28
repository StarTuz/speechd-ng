use crate::backends::SpeechBackend;
use crate::backends::espeak::EspeakBackend;
use crate::backends::Voice;
use std::io::Cursor;
use rodio::{Decoder, OutputStream, Sink};
use std::thread;
use std::sync::mpsc::{channel, Sender as MpscSender};
use tokio::sync::oneshot;

enum AudioMessage {
    Speak(String, Option<String>),
    ListVoices(oneshot::Sender<Vec<Voice>>),
}

#[derive(Clone)]
pub struct AudioEngine {
    tx: MpscSender<AudioMessage>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<AudioMessage>();
        
        // Initialize backend (future: load based on config)
        let backend: Box<dyn SpeechBackend> = Box::new(EspeakBackend::new());
        
        thread::spawn(move || {
            // Audio stream must live on this thread
            let (_stream, stream_handle) = OutputStream::try_default().expect("No audio output device found");
            
            while let Ok(msg) = rx.recv() {
                match msg {
                    AudioMessage::Speak(text, voice) => {
                         println!("Audio Thread: Synthesizing '{}' using {} (voice: {:?})", text, backend.id(), voice);
                        
                        // Handle voice option
                        let voice_id = voice.as_deref();

                        match backend.synthesize(&text, voice_id) {
                            Ok(audio_data) => {
                                let cursor = Cursor::new(audio_data);
                                match Sink::try_new(&stream_handle) {
                                    Ok(sink) => {
                                        match Decoder::new(cursor) {
                                            Ok(source) => {
                                                use rodio::Source;
                                                sink.append(source.convert_samples::<f32>());
                                                sink.detach(); 
                                            }
                                            Err(e) => eprintln!("Failed to decode: {}", e),
                                        }
                                    }
                                    Err(e) => eprintln!("Failed to create sink: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Backend error: {}", e),
                        }
                    },
                    AudioMessage::ListVoices(resp_tx) => {
                        let voices = backend.list_voices().unwrap_or_default();
                        let _ = resp_tx.send(voices);
                    }
                }
            }
        });

        Self { tx }
    }

    pub fn speak(&self, text: &str, voice: Option<String>) {
        let _ = self.tx.send(AudioMessage::Speak(text.to_string(), voice));
    }

    pub async fn list_voices(&self) -> Vec<Voice> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(AudioMessage::ListVoices(tx));
        rx.await.unwrap_or_default()
    }
}
