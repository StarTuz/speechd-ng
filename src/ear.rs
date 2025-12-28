use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};
use crate::engine::AudioEngine;
use crate::cortex::Cortex;

pub struct Ear;

impl Ear {
    pub fn new() -> Self {
        Self
    }

    pub fn listen(&self) -> String {
        println!("Ear: Manual listen triggered...");
        self.record_and_transcribe(5) 
    }

    pub fn start_autonomous_mode(&self, engine: Arc<Mutex<AudioEngine>>, cortex: Cortex) {
        let (wake_word, _enabled) = {
            let s = crate::config_loader::SETTINGS.read().unwrap();
            (s.wake_word.clone(), s.enable_wake_word)
        };
        
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let model_path = format!("{}/.cache/vosk/vosk-model-small-en-us-0.15", home);

        // Try to find the bridge near the binary first, or in Source dir
        let bridge_path = if let Ok(mut exe_path) = std::env::current_exe() {
            exe_path.pop(); // remove binary name
            let local_bridge = exe_path.join("wakeword_bridge.py");
            if local_bridge.exists() {
                local_bridge
            } else {
                // Check if we are in target/debug
                let mut p = exe_path.clone();
                p.pop(); p.pop();
                let src_bridge = p.join("src/wakeword_bridge.py");
                if src_bridge.exists() {
                    src_bridge
                } else {
                    std::path::PathBuf::from(format!("{}/Code/speechserverdaemon/src/wakeword_bridge.py", home))
                }
            }
        } else {
            std::path::PathBuf::from(format!("{}/Code/speechserverdaemon/src/wakeword_bridge.py", home))
        };

        thread::spawn(move || {
            println!("Ear: Autonomous mode active. Watching for '{}'...", wake_word);
            
            loop {
                let mut child = Command::new("python3")
                    .arg(&bridge_path)
                    .arg(&model_path)
                    .arg(&wake_word)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("Failed to start wakeword bridge");

                let mut stdin = child.stdin.take().expect("Failed to open bridge stdin");
                let stdout = child.stdout.take().expect("Failed to open bridge stdout");
                let mut bridge_reader = BufReader::new(stdout);

                let host = cpal::default_host();
                let device = host.default_input_device().expect("No input device");
                let config = device.default_input_config().expect("Failed to get config");
                
                // Use a shared atomic to stop the stream
                let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
                let running_clone = running.clone();

                let stream = device.build_input_stream(
                    &config.clone().into(),
                    move |data: &[f32], _: &_| {
                        if running_clone.load(std::sync::atomic::Ordering::SeqCst) {
                            let mut pcm = Vec::with_capacity(data.len() * 2);
                            for &sample in data {
                                let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                                pcm.extend_from_slice(&s.to_le_bytes());
                            }
                            let _ = stdin.write_all(&pcm);
                            let _ = stdin.flush();
                        }
                    },
                    |err| eprintln!("Wake word stream error: {}", err),
                    None
                ).expect("Failed to build background stream");

                stream.play().expect("Failed to start stream");

                let mut line = String::new();
                if let Ok(_) = bridge_reader.read_line(&mut line) {
                    if line.trim() == "DETECTED" {
                        println!("Ear: TRIGGERED! Switch to Command Mode.");
                        running.store(false, std::sync::atomic::Ordering::SeqCst);
                        drop(stream);
                        let _ = child.kill();

                        // 1. Notify
                        if let Ok(e) = engine.lock() {
                            e.speak("Listening.", Some("espeak:en-us".to_string()));
                        }

                        // 2. Capture Command
                        let ear = Ear::new();
                        let command = ear.record_and_transcribe(4);
                        println!("Ear: Command heard: '{}'", command);

                        if !command.trim().is_empty() {
                            // 3. Think & Respond
                            let response = tokio::runtime::Runtime::new().unwrap().block_on(async {
                                cortex.query(command).await
                            });
                            
                            if let Ok(e) = engine.lock() {
                                e.speak(&response, None);
                            }
                        }
                    }
                }
                
                // Small sleep before restarting loop if bridge exited
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    fn record_and_transcribe(&self, seconds: u64) -> String {
        let path = "/tmp/recorded_speech.wav";
        
        let host = cpal::default_host();
        let device = host.default_input_device().expect("No input device");
        let config = device.default_input_config().expect("No config");
        let sample_rate: u32 = config.sample_rate().into();

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();

        let stream = device.build_input_stream(
            &config.clone().into(),
            move |data: &[f32], _: &_| {
                if let Ok(mut b) = buffer_clone.lock() {
                    b.extend_from_slice(data);
                }
            },
            |err| eprintln!("stream error: {}", err),
            None
        ).expect("Failed to build stream");

        stream.play().unwrap();
        thread::sleep(Duration::from_secs(seconds));
        drop(stream);

        let captured_data = buffer.lock().unwrap();
        let spec = hound::WavSpec {
            channels: config.channels(),
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        if let Ok(mut writer) = hound::WavWriter::create(path, spec) {
            for &sample in captured_data.iter() {
                let _ = writer.write_sample(sample);
            }
            let _ = writer.finalize();
        }

        self.transcribe_cli(path).unwrap_or_else(|e| format!("STT Error: {}", e))
    }

    fn transcribe_cli(&self, path: &str) -> Result<String, String> {
        let txt_path = path.replace(".wav", ".txt");
        let output = Command::new("vosk-transcriber")
            .arg("-i").arg(path)
            .arg("-o").arg(&txt_path)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                return std::fs::read_to_string(&txt_path)
                    .map_err(|e| format!("Read error: {}", e));
            }
        }
        Err("Vosk transcriber failed".to_string())
    }
}
