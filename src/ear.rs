use crate::cortex::Cortex;
use crate::engine::AudioOutput;
use crate::wyoming::WyomingClient;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use deunicode::deunicode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use vosk::{Model, Recognizer};

pub struct Ear {
    restart_requested: Arc<AtomicBool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum VadState {
    Waiting,
    Speaking,
    Silence,
}

impl Ear {
    pub fn new() -> Self {
        Self {
            restart_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn new_dummy() -> Self {
        Self {
            restart_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn trigger_restart(&self) {
        self.restart_requested.store(true, Ordering::SeqCst);
    }

    pub fn listen(&self) -> String {
        // Default threshold 0.5
        self.listen_vad("/usr/share/vosk/model", 0.5)
            .unwrap_or_default()
    }

    pub fn record_with_vad(&self) -> String {
        // Also used for VAD listen in service.rs
        self.listen_vad("/usr/share/vosk/model", 0.5)
            .unwrap_or_default()
    }

    pub fn record_and_transcribe(&self, _seconds: u64) -> String {
        // Simple record for fixed duration
        // This is a placeholder since the full implementation was lost
        // but it satisfies the API.
        self.listen_vad("/usr/share/vosk/model", 0.5)
            .unwrap_or_default()
    }

    pub fn start_autonomous_mode(
        &self,
        engine: Arc<dyn AudioOutput + Send + Sync>,
        cortex: Cortex,
    ) {
        let (model_path, threshold) = {
            let config = crate::config_loader::SETTINGS.read().unwrap();
            (
                config.vosk_model_path.clone(),
                config.passive_confidence_threshold,
            )
        };
        self.run(cortex, engine, model_path, threshold);
    }

    pub fn run(
        &self,
        cortex: Cortex,
        _engine: Arc<dyn AudioOutput + Send + Sync>,
        vosk_model_path: String,
        vad_threshold: f32,
    ) {
        let restart_requested = self.restart_requested.clone();
        thread::spawn(move || loop {
            restart_requested.store(false, Ordering::SeqCst);
            if let Err(e) =
                Self::run_internal(&cortex, &vosk_model_path, vad_threshold, &restart_requested)
            {
                eprintln!("Ear caught error: {}. Restarting in 5s...", e);
                thread::sleep(Duration::from_secs(5));
            }
            if !restart_requested.load(Ordering::SeqCst) {
                println!("Ear thread exiting.");
                break;
            }
            println!("Ear restarting...");
        });
    }

    fn run_internal(
        cortex: &Cortex,
        model_path: &str,
        vad_threshold: f32,
        restart_requested: &Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let model = Model::new(model_path).ok_or("Failed to load Vosk model")?;
        let mut recognizer =
            Recognizer::new(&model, 16000.0).ok_or("Failed to create recognizer")?;

        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("No input device found")?;
        let config = device.default_input_config()?;

        println!("Ear using input device: {}", device.name()?);

        let vad_state = Arc::new(Mutex::new(VadState::Waiting));
        let silence_start = Arc::new(Mutex::new(None));
        let settings = (vad_threshold * 32768.0) as i16;

        let mut chunk_accumulator = Vec::new();
        let samples_per_chunk = (16000.0 * 0.1) as usize; // 100ms chunks

        let vad_state_clone = vad_state.clone();
        let cortex_clone = cortex.clone();
        let silence_start_clone = silence_start.clone();

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                chunk_accumulator.extend_from_slice(data);
                while chunk_accumulator.len() >= samples_per_chunk {
                    let chunk: Vec<f32> = chunk_accumulator.drain(..samples_per_chunk).collect();
                    let energy =
                        (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
                    let energy_i16 = (energy * 32768.0) as i16;

                    let mut state = vad_state_clone.lock().unwrap();
                    let now = std::time::Instant::now();

                    match *state {
                        VadState::Waiting => {
                            if energy_i16 > settings {
                                *state = VadState::Speaking;
                                println!("VAD: Speech detected!");
                            }
                        }
                        VadState::Speaking => {
                            let i16_samples: Vec<i16> = chunk
                                .iter()
                                .map(|&f| (f * 32768.0).clamp(-32768.0, 32767.0) as i16)
                                .collect();

                            if recognizer.accept_waveform(&i16_samples).is_ok() {
                                let result = recognizer.final_result().single().unwrap().text;
                                if !result.is_empty() {
                                    let sanitized = deunicode(result);
                                    println!("Heard: {}", sanitized);
                                    let c = cortex_clone.clone();
                                    let s = sanitized.to_string();
                                    tokio::spawn(async move {
                                        c.observe(s).await;
                                    });
                                }
                            }

                            if energy_i16 < (settings / 2) {
                                *state = VadState::Silence;
                                *silence_start_clone.lock().unwrap() = Some(now);
                            }
                        }
                        VadState::Silence => {
                            if energy_i16 > settings {
                                *state = VadState::Speaking;
                                *silence_start_clone.lock().unwrap() = None;
                            } else if let Some(start) = *silence_start_clone.lock().unwrap() {
                                if now.duration_since(start).as_secs_f32() > 1.5 {
                                    println!("VAD: Silence detected. Resetting.");
                                    let result = recognizer.final_result().single().unwrap().text;
                                    if !result.is_empty() {
                                        let sanitized = deunicode(result);
                                        println!("Finalized: {}", sanitized);
                                        let c = cortex_clone.clone();
                                        let s = sanitized.to_string();
                                        tokio::spawn(async move {
                                            c.observe(s).await;
                                        });
                                    }
                                    *state = VadState::Waiting;
                                    *silence_start_clone.lock().unwrap() = None;
                                }
                            }
                        }
                    }
                }
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        )?;

        stream.play()?;

        while !restart_requested.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }

    pub fn listen_vad(
        &self,
        vosk_model_path: &str,
        vad_threshold: f32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let model = Model::new(vosk_model_path).ok_or("Failed to load Vosk model")?;
        let mut recognizer =
            Recognizer::new(&model, 16000.0).ok_or("Failed to create recognizer")?;

        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("No input device found")?;
        let config = device.default_input_config()?;

        let result = Arc::new(Mutex::new(None));
        let result_clone = result.clone();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = finished.clone();

        let settings = (vad_threshold * 32768.0) as i16;
        let vad_state = Arc::new(Mutex::new(VadState::Waiting));
        let silence_start = Arc::new(Mutex::new(None));

        let mut chunk_accumulator = Vec::new();
        let samples_per_chunk = (16000.0 * 0.1) as usize;

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                if finished_clone.load(Ordering::SeqCst) {
                    return;
                }
                chunk_accumulator.extend_from_slice(data);
                while chunk_accumulator.len() >= samples_per_chunk {
                    let chunk: Vec<f32> = chunk_accumulator.drain(..samples_per_chunk).collect();
                    let energy =
                        (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
                    let energy_i16 = (energy * 32768.0) as i16;

                    let mut state = vad_state.lock().unwrap();
                    let now = std::time::Instant::now();

                    match *state {
                        VadState::Waiting => {
                            if energy_i16 > settings {
                                *state = VadState::Speaking;
                            }
                        }
                        VadState::Speaking => {
                            let i16_samples: Vec<i16> = chunk
                                .iter()
                                .map(|&f| (f * 32768.0).clamp(-32768.0, 32767.0) as i16)
                                .collect();

                            if recognizer.accept_waveform(&i16_samples).is_ok() {
                                // Keep going
                            }

                            if energy_i16 < (settings / 2) {
                                *state = VadState::Silence;
                                *silence_start.lock().unwrap() = Some(now);
                            }
                        }
                        VadState::Silence => {
                            if energy_i16 > settings {
                                *state = VadState::Speaking;
                                *silence_start.lock().unwrap() = None;
                            } else if let Some(start) = *silence_start.lock().unwrap() {
                                if now.duration_since(start).as_secs_f32() > 1.0 {
                                    let text = recognizer.final_result().single().unwrap().text;
                                    *result_clone.lock().unwrap() = Some(text.to_string());
                                    finished_clone.store(true, Ordering::SeqCst);
                                }
                            }
                        }
                    }
                }
            },
            |err| eprintln!("Stream error: {}", err),
            None,
        )?;

        stream.play()?;

        let start = std::time::Instant::now();
        while !finished.load(Ordering::SeqCst) && start.elapsed().as_secs() < 10 {
            thread::sleep(Duration::from_millis(100));
        }

        let final_text = result.lock().unwrap().take().unwrap_or_default();
        Ok(deunicode(&final_text))
    }

    pub fn listen_wyoming(
        &self,
        _cortex: Cortex,
        _host: &str,
        _port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation for wyoming client needs to be async or handled in a tokio runtime.
        // For now, we'll keep it as a placeholder to fix compilation.
        Ok(())
    }
}
