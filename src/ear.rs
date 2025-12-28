use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::process::Command;

pub struct Ear;

impl Ear {
    pub fn new() -> Self {
        Self
    }

    pub fn listen(&self) -> String {
        println!("Ear: Starting to listen...");
        
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => return "Error: No input device found".to_string(),
        };

        println!("Ear: Found input device: {}", device.name().unwrap_or("Unknown".into()));

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => return format!("Error getting config: {}", e),
        };

        println!("Ear: Input config: {:?}", config);

        // Buffer to store recorded samples
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();

        // Error callback
        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        // Data callback
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.clone().into(),
                move |data: &[f32], _: &_| {
                    if let Ok(mut b) = buffer_clone.lock() {
                        b.extend_from_slice(data);
                    }
                },
                err_fn,
                None 
            ),
            _ => return "Error: Only F32 sample format supported for now".to_string(),
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => return format!("Error building stream: {}", e),
        };

        match stream.play() {
            Ok(_) => println!("Ear: Recording..."),
            Err(e) => return format!("Error playing stream: {}", e),
        }

        // Record for 3 seconds
        thread::sleep(Duration::from_secs(3));
        
        drop(stream); // Stop recording

        // Save to WAV
        let path = "/tmp/recorded_speech.wav";
        let captured_data = buffer.lock().unwrap();
        
        let spec = hound::WavSpec {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let wav_msg = match hound::WavWriter::create(path, spec) {
            Ok(mut writer) => {
                for &sample in captured_data.iter() {
                    let _ = writer.write_sample(sample);
                }
                let _ = writer.finalize();
                format!("Recorded {} samples to {}", captured_data.len(), path)
            },
            Err(e) => format!("Error saving WAV: {}", e)
        };

        // Attempt Transcription
        match self.transcribe(path) {
            Ok(text) => format!("{}\nTranscript: {}", wav_msg, text),
            Err(e) => format!("{}\nSTT Error: {}", wav_msg, e),
        }
    }

    fn transcribe(&self, path: &str) -> Result<String, String> {
        // Try standard 'whisper' command (OpenAI Python or cpp)
        // We assume 'whisper <file> --output_format txt --output_dir /tmp' 
        // Or 'whisper <file> --model tiny --language en'
        
        // Simple check if whisper exists
        let output = Command::new("whisper")
            .arg(path)
            .arg("--model")
            .arg("tiny")
            .arg("--output_format")
            .arg("txt")
            .arg("--output_dir")
            .arg("/tmp")
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    // Read the output file
                    // Whisper typically creates /tmp/recorded_speech.txt
                    let txt_path = path.replace(".wav", ".txt");
                    std::fs::read_to_string(&txt_path)
                        .map_err(|e| format!("Could not read transcript: {}", e))
                } else {
                    let err = String::from_utf8_lossy(&out.stderr);
                    Err(format!("Whisper failed: {}", err))
                }
            },
            Err(_) => {
                // Try 'main' (whisper.cpp)
                // main -f <file> -m <model> -otxt
                // This is too specific. Just fail gracefully.
                Err("STT Engine 'whisper' not found in PATH. Please install openai-whisper.".to_string())
            }
        }
    }
}
