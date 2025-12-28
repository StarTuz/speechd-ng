use std::process::Command;
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
        
        thread::spawn(move || {
            // Audio stream must live on this thread
            let (_stream, stream_handle) = OutputStream::try_default().expect("No audio output device found");
            
            while let Ok(text) = rx.recv() {
                println!("Audio Thread: Synthesizing '{}'", text);
                
                let output = Command::new("espeak-ng")
                    .arg("--stdout")
                    .arg(&text)
                    .output();
                
                match output {
                    Ok(out) => {
                        if out.status.success() {
                            let cursor = Cursor::new(out.stdout);
                            match Sink::try_new(&stream_handle) {
                                Ok(sink) => {
                                    match Decoder::new(cursor) {
                                        Ok(source) => {
                                            use rodio::Source;
                                            sink.append(source.convert_samples::<f32>());
                                            // We must wait for the sound to finish or at least not drop the sink immediately
                                            // sink.sleep_until_end(); 
                                            // But if we sleep, we block the next message.
                                            // Ideally we just detach(), but the stream must stay alive (it does, in this loop)
                                            sink.detach(); 
                                        }
                                        Err(e) => eprintln!("Failed to decode: {}", e),
                                    }
                                }
                                Err(e) => eprintln!("Failed to create sink: {}", e),
                            }
                        } else {
                            eprintln!("espeak error: {:?}", String::from_utf8_lossy(&out.stderr));
                        }
                    }
                    Err(e) => eprintln!("Failed to run espeak: {}", e),
                }
            }
        });

        Self { tx }
    }

    pub fn speak(&self, text: &str) {
        let _ = self.tx.send(text.to_string());
    }
}
