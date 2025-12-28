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
                println!("Ear: Loop iteration started...");
                let host = cpal::default_host();
                println!("Ear: Host acquired.");
                
                let device = {
                    let mut selected = None;
                    if let Ok(devices) = host.input_devices() {
                        for d in devices {
                            let name = d.name().unwrap_or_else(|_| "Unknown".into());
                            println!("Ear: Found input device: {}", name);
                            
                            // Skip clearly non-mic or dummy devices
                            if name == "null" || name == "default" || name.contains("Monitor") || name == "jack" {
                                continue;
                            }

                            // Prioritize devices that look like real physical mics
                            if name.contains("CARD=") || name.contains("Headset") || name.contains("Built-in") {
                                selected = Some(d);
                                break; // Take the first good physical device
                            }
                            
                            if selected.is_none() {
                                selected = Some(d);
                            }
                        }
                    }
                    selected.or_else(|| host.default_input_device()).expect("No input device")
                };

                println!("Ear: Device acquired: {:?}, Backend: {:?}", device.name().ok(), host.id());
                let config = device.default_input_config().expect("Failed to get config");
                println!("Ear: Config acquired.");
                
                let sample_rate: u32 = config.sample_rate().into();
                let sample_rate_str = sample_rate.to_string();
                let channels = config.channels();
                println!("Ear: Microdevice: {}, Sample Rate: {}, Channels: {}", 
                    device.name().unwrap_or_else(|_| "Unknown".into()),
                    sample_rate,
                    channels
                );

                println!("Ear: Starting bridge at path: {:?}", bridge_path);

                let mut child = Command::new("python3")
                    .arg(&bridge_path)
                    .arg(&model_path)
                    .arg(&wake_word)
                    .arg(&sample_rate_str)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to start wakeword bridge");

                let mut stdin = child.stdin.take().expect("Failed to open bridge stdin");
                let stdout = child.stdout.take().expect("Failed to open bridge stdout");
                let stderr = child.stderr.take().expect("Failed to open bridge stderr");
                let mut bridge_reader = BufReader::new(stdout);

                // Spawn a thread to forward stderr to our logs
                thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines() {
                        if let Ok(l) = line {
                            println!("Ear [Bridge Error]: {}", l);
                        }
                    }
                });
                
                // Use a shared atomic to stop the stream
                let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
                let running_clone = running.clone();

                let mut last_pulse = std::time::Instant::now();

                let stream = device.build_input_stream(
                    &config.clone().into(),
                    move |data: &[f32], _: &_| {
                        if running_clone.load(std::sync::atomic::Ordering::SeqCst) {
                            if last_pulse.elapsed() > Duration::from_secs(5) {
                                println!("Ear: Audio Flow Pulse (Callback active)");
                                last_pulse = std::time::Instant::now();
                            }
                            let mut pcm = Vec::with_capacity(data.len() / (channels as usize) * 2);
                            // Only take the first channel if it's multi-channel
                            for chunk in data.chunks_exact(channels as usize) {
                                let sample = chunk[0];
                                let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                                pcm.extend_from_slice(&s.to_le_bytes());
                            }
                            let _ = stdin.write_all(&pcm);
                            let _ = stdin.flush();
                        }
                    },
                    |err| println!("Wake word stream error: {}", err),
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
                        // 1. Notify
                        if let Ok(e) = engine.lock() {
                            tokio::runtime::Runtime::new().unwrap().block_on(async {
                                e.speak_blocking("Listening.", None).await;
                            });
                        }

                        // 2. Capture Command (using VAD for natural recording)
                        let ear = Ear::new();
                        let command = ear.record_with_vad();
                        println!("Ear: Command heard (VAD): '{}'", command);

                        if !command.trim().is_empty() {
                            // 3. Think & Respond
                            let response = tokio::runtime::Runtime::new().unwrap().block_on(async {
                                cortex.query_with_asr(command.clone(), command).await
                            });
                            
                            if let Ok(e) = engine.lock() {
                                e.speak(&response, None);
                            }
                        } else {
                            // No speech detected
                            if let Ok(e) = engine.lock() {
                                e.speak("I didn't hear anything.", None);
                            }
                        }
                    }
                }
                
                // Small sleep before restarting loop if bridge exited
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    /// Record audio for specified duration and transcribe it (fallback, fixed duration)
    pub fn record_and_transcribe(&self, seconds: u64) -> String {
        let path = "/tmp/recorded_speech.wav";
        
        let host = cpal::default_host();
        let device = {
            let mut selected = None;
            if let Ok(devices) = host.input_devices() {
                for d in devices {
                    let name = d.name().unwrap_or_else(|_| "Unknown".into());
                    if name == "null" || name == "default" || name.contains("Monitor") || name == "jack" {
                        continue;
                    }
                    if name.contains("CARD=") || name.contains("Headset") || name.contains("Built-in") {
                        selected = Some(d);
                        break;
                    }
                    if selected.is_none() {
                        selected = Some(d);
                    }
                }
            }
            selected.or_else(|| host.default_input_device()).expect("No input device")
        };

        println!("Ear: Recording command from device: {:?}", device.name().ok());
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
            move |err| {
                eprintln!("Ear: Recording stream error: {}", err);
            },
            None
        ).map_err(|e| {
            eprintln!("Ear: Failed to build recording stream: {}", e);
            e
        }).expect("Failed to build stream");

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

    /// Record audio with VAD (Voice Activity Detection)
    /// Starts recording when speech is detected, stops after silence
    pub fn record_with_vad(&self) -> String {
        let path = "/tmp/recorded_speech_vad.wav";
        
        // Get VAD settings
        let (speech_threshold, silence_threshold, silence_duration_ms, max_duration_ms) = {
            let settings = crate::config_loader::SETTINGS.read().unwrap();
            (
                settings.vad_speech_threshold,
                settings.vad_silence_threshold,
                settings.vad_silence_duration_ms,
                settings.vad_max_duration_ms,
            )
        };
        
        let host = cpal::default_host();
        let device = host.default_input_device().expect("No input device");
        
        println!("Ear: VAD recording from device: {:?}", device.name().ok());
        println!("Ear: VAD thresholds - speech: {}, silence: {}, timeout: {}ms, max: {}ms", 
            speech_threshold, silence_threshold, silence_duration_ms, max_duration_ms);
        
        let config = device.default_input_config().expect("No config");
        let sample_rate: u32 = config.sample_rate().into();
        let channels = config.channels();
        
        // Shared state for VAD
        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let vad_state = Arc::new(Mutex::new(VadState::Waiting));
        let speech_started = Arc::new(Mutex::new(std::time::Instant::now()));
        let silence_started = Arc::new(Mutex::new(Option::<std::time::Instant>::None));
        
        let buffer_clone = buffer.clone();
        let vad_state_clone = vad_state.clone();
        let speech_started_clone = speech_started.clone();
        let silence_started_clone = silence_started.clone();
        
        // Calculate samples per chunk for energy calculation (10ms chunks)
        let samples_per_chunk = (sample_rate as usize * channels as usize) / 100;
        let mut chunk_buffer = Vec::with_capacity(samples_per_chunk);
        
        let stream = device.build_input_stream(
            &config.clone().into(),
            move |data: &[f32], _: &_| {
                chunk_buffer.extend_from_slice(data);
                
                // Process complete chunks
                while chunk_buffer.len() >= samples_per_chunk {
                    let chunk: Vec<f32> = chunk_buffer.drain(..samples_per_chunk).collect();
                    
                    // Calculate RMS energy (convert f32 to i16 scale for threshold comparison)
                    let energy: f32 = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
                    let energy_i16 = (energy * 32768.0) as i16;
                    
                    let mut state = vad_state_clone.lock().unwrap();
                    let now = std::time::Instant::now();
                    
                    match *state {
                        VadState::Waiting => {
                            if energy_i16 > speech_threshold {
                                println!("Ear: [VAD] Speech detected! (energy: {})", energy_i16);
                                *state = VadState::Speaking;
                                *speech_started_clone.lock().unwrap() = now;
                                // Start buffering audio
                                if let Ok(mut b) = buffer_clone.lock() {
                                    b.extend_from_slice(&chunk);
                                }
                            }
                        }
                        VadState::Speaking => {
                            // Always buffer audio during speaking state
                            if let Ok(mut b) = buffer_clone.lock() {
                                b.extend_from_slice(&chunk);
                            }
                            
                            // Check for silence
                            if energy_i16 < silence_threshold {
                                let mut silence = silence_started_clone.lock().unwrap();
                                if silence.is_none() {
                                    *silence = Some(now);
                                } else if let Some(start) = *silence {
                                    if now.duration_since(start).as_millis() >= silence_duration_ms as u128 {
                                        println!("Ear: [VAD] Silence detected, ending recording");
                                        *state = VadState::Done;
                                    }
                                }
                            } else {
                                // Reset silence timer if speech resumes
                                *silence_started_clone.lock().unwrap() = None;
                            }
                            
                            // Check max duration
                            let speech_start = *speech_started_clone.lock().unwrap();
                            if now.duration_since(speech_start).as_millis() >= max_duration_ms as u128 {
                                println!("Ear: [VAD] Max duration reached");
                                *state = VadState::Done;
                            }
                        }
                        VadState::Done => {
                            // Stop processing
                        }
                    }
                }
            },
            move |err| {
                eprintln!("Ear: VAD stream error: {}", err);
            },
            None
        ).expect("Failed to build VAD stream");
        
        stream.play().unwrap();
        
        // Wait for VAD to complete or timeout
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(max_duration_ms + 5000); // Extra 5s for startup
        
        loop {
            thread::sleep(Duration::from_millis(50));
            
            let state = vad_state.lock().unwrap();
            if *state == VadState::Done {
                break;
            }
            
            if start.elapsed() > timeout {
                println!("Ear: [VAD] Timeout waiting for speech");
                break;
            }
        }
        
        drop(stream);
        
        // Write captured audio to file
        let captured_data = buffer.lock().unwrap();
        
        if captured_data.is_empty() {
            println!("Ear: [VAD] No audio captured");
            return String::new();
        }
        
        println!("Ear: [VAD] Captured {} samples", captured_data.len());
        
        let spec = hound::WavSpec {
            channels,
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

// VAD State Machine
#[derive(Clone, Copy, PartialEq, Debug)]
enum VadState {
    Waiting,   // Waiting for speech to start
    Speaking,  // Speech detected, recording
    Done,      // Recording complete
}
