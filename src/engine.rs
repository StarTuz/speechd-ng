use crate::backends::SpeechBackend;
use crate::backends::espeak::EspeakBackend;
use std::io::Cursor;
use rodio::{Decoder, OutputStream, Sink};
use std::thread;
use std::sync::mpsc::{channel, Sender};

#[derive(Clone)]
pub struct AudioEngine {
    tx: Sender<String>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<String>();
        
        // Initialize backend (future: load based on config)
        let backend: Box<dyn SpeechBackend> = Box::new(EspeakBackend::new());
        
        thread::spawn(move || {
            // Audio stream must live on this thread
            let (_stream, stream_handle) = OutputStream::try_default().expect("No audio output device found");
            
            while let Ok(text) = rx.recv() {
                println!("Audio Thread: Synthesizing '{}' using {}", text, backend.id());
                
                match backend.synthesize(&text) {
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
            }
        });

        Self { tx }
    }

    pub fn speak(&self, text: &str) {
        let _ = self.tx.send(text.to_string());
    }
}
