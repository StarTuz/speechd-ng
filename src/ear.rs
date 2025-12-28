use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::process::{Command, Stdio};
use std::io::{Write, BufRead, BufReader};
use wait_timeout::ChildExt;
use crate::engine::AudioEngine;
use crate::cortex::Cortex;

/// Helper to find bridge scripts in dev, local, or system paths
fn find_bridge_script(script_name: &str) -> std::path::PathBuf {
    // 1. Next to executable
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        let local_path = exe_path.join(script_name);
        if local_path.exists() { return local_path; }
        
        // 2. Development (target/debug/../../src)
        let mut p = exe_path.clone();
        p.pop(); p.pop();
        let src_path = p.join("src").join(script_name);
        if src_path.exists() { return src_path; }
    }

    // 3. System install path
    let sys_path = std::path::PathBuf::from("/usr/lib/speechd-ng").join(script_name);
    if sys_path.exists() { return sys_path; }

    // 4. Legacy fallback
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(format!("{}/Code/speechserverdaemon/src/{}", home, script_name))
}

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

        // Find bridge script
        let bridge_path = find_bridge_script("wakeword_bridge.py");

        thread::spawn(move || {
            println!("Ear: Autonomous mode active. Watching for '{}'...", wake_word);
            
            loop {
                println!("Ear: Loop iteration started...");
                let host = cpal::default_host();
                println!("Ear: Host acquired.");
                
                let device_resource = {
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
                    selected.or_else(|| host.default_input_device())
                };

                let device = if let Some(d) = device_resource {
                    d
                } else {
                    eprintln!("Ear: No input device found. Standing by (will retry in 30s)...");
                    thread::sleep(Duration::from_secs(30));
                    continue;
                };

                println!("Ear: Device acquired: {:?}, Backend: {:?}", device.name().ok(), host.id());
                let config = match device.default_input_config() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Ear: Failed to get input config: {}. Standing by...", e);
                        thread::sleep(Duration::from_secs(30));
                        continue;
                    }
                };
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
            if let Some(d) = selected.or_else(|| host.default_input_device()) {
                d
            } else {
                return "Error: No input device available".to_string();
            }
        };

        println!("Ear: Recording command from device: {:?}", device.name().ok());
        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(_) => return "Error: Failed to get input config".to_string(),
        };
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
        let device = if let Some(d) = host.default_input_device() {
            d
        } else {
            return "Error: No input device found".to_string();
        };
        
        println!("Ear: VAD recording from device: {:?}", device.name().ok());
        println!("Ear: VAD thresholds - speech: {}, silence: {}, timeout: {}ms, max: {}ms", 
            speech_threshold, silence_threshold, silence_duration_ms, max_duration_ms);
        
        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(_) => return "Error: No audio config available".to_string(),
        };
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

    /// Transcribe audio file using configured STT backend
    fn transcribe_cli(&self, path: &str) -> Result<String, String> {
        let stt_backend = {
            crate::config_loader::SETTINGS.read()
                .map(|s| s.stt_backend.clone())
                .unwrap_or_else(|_| "vosk".to_string())
        };

        match stt_backend.as_str() {
            "wyoming" => self.transcribe_wyoming(path),
            _ => self.transcribe_vosk(path),
        }
    }

    /// Transcribe using Vosk CLI
    fn transcribe_vosk(&self, path: &str) -> Result<String, String> {
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

    /// Transcribe using Wyoming bridge (streams to Wyoming-Whisper server)
    fn transcribe_wyoming(&self, wav_path: &str) -> Result<String, String> {
        let (host, port) = {
            let settings = crate::config_loader::SETTINGS.read()
                .map_err(|_| "Settings lock error".to_string())?;
            (settings.wyoming_host.clone(), settings.wyoming_port)
        };

        println!("Ear: [Wyoming] Transcribing via {}:{}", host, port);

        // Find the Wyoming bridge script
        let bridge_path = find_bridge_script("wyoming_bridge.py");

        // Convert WAV to raw PCM 16-bit mono 16kHz for Wyoming
        let pcm_path = wav_path.replace(".wav", ".pcm");
        let ffmpeg_result = Command::new("ffmpeg")
            .args(["-y", "-i", wav_path, "-f", "s16le", "-ar", "16000", "-ac", "1", &pcm_path])
            .stderr(Stdio::null())
            .output();

        if ffmpeg_result.is_err() {
            return Err("FFmpeg conversion failed".to_string());
        }

        // Read PCM data and pipe to Wyoming bridge
        let pcm_data = std::fs::read(&pcm_path).map_err(|e| format!("Read PCM error: {}", e))?;
        let _ = std::fs::remove_file(&pcm_path); // Cleanup

        let mut child = Command::new("python3")
            .arg(&bridge_path)
            .arg("--host").arg(&host)
            .arg("--port").arg(port.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start Wyoming bridge: {}", e))?;

        // Write PCM data to bridge stdin
        if let Some(ref mut stdin) = child.stdin {
            let _ = stdin.write_all(&pcm_data);
        }
        drop(child.stdin.take());

        // Read transcript from stdout
        let output = child.wait_with_output().map_err(|e| format!("Wait error: {}", e))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse "TRANSCRIPT: <text>" from output
        for line in stdout.lines() {
            if line.starts_with("TRANSCRIPT:") {
                let transcript = line.trim_start_matches("TRANSCRIPT:").trim();
                println!("Ear: [Wyoming] Got transcript: '{}'", transcript);
                return Ok(transcript.to_string());
            }
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            println!("Ear: [Wyoming] Bridge stderr: {}", stderr);
        }

        Err("Wyoming transcription failed".to_string())
    }
}

// VAD State Machine
#[derive(Clone, Copy, PartialEq, Debug)]
enum VadState {
    Waiting,   // Waiting for speech to start
    Speaking,  // Speech detected, recording
    Done,      // Recording complete
}

