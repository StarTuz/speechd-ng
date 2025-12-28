mod engine;
mod cortex;
mod config_loader;
mod security;
mod backends;
mod ear;
mod ssip;
mod fingerprint;
use engine::AudioEngine;
use cortex::Cortex;
use ear::Ear;
use fingerprint::Fingerprint;
use security::SecurityAgent;
use std::error::Error;
use std::future::pending;
use std::sync::{Arc, Mutex};
use zbus::{interface, connection::Builder, message::Header};

struct SpeechService {
    engine: Arc<Mutex<AudioEngine>>,
    cortex: Cortex,
    ear: Arc<Mutex<Ear>>,
    fingerprint: Fingerprint,
}

#[interface(name = "org.speech.Service")]
impl SpeechService {
    async fn speak(&self, #[zbus(header)] _header: Header<'_>, text: String) {
        println!("Received speak request: {}", text);
        
        let audio_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);
        
        if audio_enabled {
            if let Ok(engine) = self.engine.lock() {
                engine.speak(&text, None);
            }
        }
        let ai_enabled = config_loader::SETTINGS.read().map(|s| s.enable_ai).unwrap_or(true);
        if ai_enabled {
            self.cortex.observe(text).await;
        }
    }

    async fn speak_voice(&self, #[zbus(header)] _header: Header<'_>, text: String, voice: String) {
         println!("Received speak request (voice: {}): {}", voice, text);
         
         let audio_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);

         if audio_enabled {
             if let Ok(engine) = self.engine.lock() {
                 engine.speak(&text, Some(voice));
             }
         }
         let ai_enabled = config_loader::SETTINGS.read().map(|s| s.enable_ai).unwrap_or(true);
         if ai_enabled {
             self.cortex.observe(text).await;
         }
    }

    async fn list_voices(&self) -> Vec<(String, String)> {
        let engine = if let Ok(engine) = self.engine.lock() {
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = engine {
             let list = engine.list_voices().await;
             list.into_iter().map(|v| (v.id, v.name)).collect()
        } else {
             Vec::new()
        }
    }

    async fn list_downloadable_voices(&self) -> Vec<(String, String)> {
        let engine = if let Ok(engine) = self.engine.lock() {
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = engine {
             let list = engine.list_downloadable_voices().await;
             list.into_iter().map(|v| (v.id, format!("{} [{}]", v.name, v.language))).collect()
        } else {
             Vec::new()
        }
    }

    async fn download_voice(&self, #[zbus(header)] header: Header<'_>, voice_id: String) -> String {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.manage").await {
            return format!("Access Denied: {}", e);
        }

        let engine = if let Ok(engine) = self.engine.lock() {
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = engine {
            match engine.download_voice(voice_id).await {
                Ok(_) => "Success".to_string(),
                Err(e) => format!("Error: {}", e),
            }
        } else {
             "Error: Engine locked".to_string()
        }
    }

    async fn think(&self, #[zbus(header)] header: Header<'_>, query: String) -> String {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.think").await {
            eprintln!("Access Denied: {}", e);
            return "Access Denied".to_string();
        }

        let ai_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_ai)
            .unwrap_or(true);

        if !ai_enabled {
            return "AI Disabled".to_string();
        }

        println!("Received thought query: {}", query);
        self.cortex.query(query).await
    }

    async fn listen(&self, #[zbus(header)] header: Header<'_>) -> String {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.listen").await {
            eprintln!("Access Denied: {}", e);
            return "Access Denied".to_string();
        }

        println!("Received listen request");
        
        let ear = self.ear.clone();
        let result = tokio::task::spawn_blocking(move || {
            if let Ok(ear_guard) = ear.lock() {
                ear_guard.listen()
            } else {
                "Error: Ear locked".to_string()
            }
        }).await;

        match result {
            Ok(s) => s,
            Err(e) => format!("Error joining audio task: {}", e),
        }
    }

    /// Listen with Voice Activity Detection (Phase 12)
    /// Waits for speech, records until silence, then transcribes
    async fn listen_vad(&self, #[zbus(header)] header: Header<'_>) -> String {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.listen").await {
            eprintln!("Access Denied: {}", e);
            return "Access Denied".to_string();
        }

        println!("Received VAD listen request");
        
        let ear = self.ear.clone();
        let result = tokio::task::spawn_blocking(move || {
            if let Ok(ear_guard) = ear.lock() {
                ear_guard.record_with_vad()
            } else {
                "Error: Ear locked".to_string()
            }
        }).await;

        match result {
            Ok(s) => s,
            Err(e) => format!("Error joining audio task: {}", e),
        }
    }

    // ========== Phase 9: Voice Training API ==========

    /// Add a manual voice correction (heard -> meant)
    /// This is used when the user knows what ASR mishears
    async fn add_correction(&self, heard: String, meant: String) -> bool {
        println!("Adding manual correction: '{}' -> '{}'", heard, meant);
        self.fingerprint.add_manual_correction(heard, meant)
    }

    /// Undo the last correction (manual or passive)
    async fn rollback_last_correction(&self) -> bool {
        self.fingerprint.rollback_last_correction()
    }

    /// Train a word by recording user speech and learning what ASR hears
    /// Returns (what_asr_heard, success)
    async fn train_word(&self, #[zbus(header)] header: Header<'_>, expected: String, duration_secs: u32) -> (String, bool) {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.train").await {
            eprintln!("Access Denied for TrainWord: {}", e);
            return ("Access Denied".to_string(), false);
        }

        println!("Training word '{}' for {} seconds...", expected, duration_secs);
        
        let ear = self.ear.clone();
        let fingerprint = self.fingerprint.clone();
        let expected_clone = expected.clone();
        
        let result = tokio::task::spawn_blocking(move || {
            if let Ok(ear_guard) = ear.lock() {
                // Record and transcribe
                let heard = ear_guard.record_and_transcribe(duration_secs as u64);
                let heard_trimmed = heard.trim().to_string();
                
                if heard_trimmed.is_empty() {
                    return ("[no speech detected]".to_string(), false);
                }
                
                // Learn the correction
                let success = fingerprint.add_manual_correction(heard_trimmed.clone(), expected_clone);
                (heard_trimmed, success)
            } else {
                ("Error: Ear locked".to_string(), false)
            }
        }).await;

        match result {
            Ok((heard, success)) => {
                // Audio feedback on success
                if success {
                    let feedback = format!("I heard {}. I'll remember that means {}.", heard, expected);
                    if let Ok(engine) = self.engine.lock() {
                        engine.speak(&feedback, None);
                    }
                }
                (heard, success)
            },
            Err(e) => (format!("Error: {}", e), false),
        }
    }

    /// Get fingerprint statistics (manual_patterns, passive_patterns, command_count)
    async fn get_fingerprint_stats(&self) -> (u32, u32, u32) {
        self.fingerprint.get_stats()
    }

    /// List all learned patterns (for debugging/UI)
    async fn list_patterns(&self) -> Vec<(String, String, String)> {
        self.fingerprint.get_all_patterns()
            .into_iter()
            .map(|(heard, meant, conf, source)| {
                (heard, meant, format!("{:.0}% ({})", conf * 100.0, source))
            })
            .collect()
    }

    // ========== Phase 10: Pattern Import/Export ==========

    /// Export fingerprint to a file
    /// Returns true if successful
    async fn export_fingerprint(&self, path: String) -> bool {
        println!("Exporting fingerprint to: {}", path);
        self.fingerprint.export_to_path(&path)
    }

    /// Import fingerprint from a file
    /// If merge=true, adds new patterns without overwriting existing
    /// If merge=false, replaces current fingerprint entirely
    /// Returns total pattern count after import
    async fn import_fingerprint(&self, path: String, merge: bool) -> u32 {
        println!("Importing fingerprint from: {} (merge={})", path, merge);
        self.fingerprint.import_from_path(&path, merge)
    }

    /// Get the path to the fingerprint data file
    async fn get_fingerprint_path(&self) -> String {
        self.fingerprint.get_path()
    }

    // ========== Phase 11: Ignored Commands Tracking ==========

    /// Get all ignored commands (heard, timestamp, context)
    async fn get_ignored_commands(&self) -> Vec<(String, String, String)> {
        self.fingerprint.get_ignored_commands()
    }

    /// Clear all ignored commands
    /// Returns count of commands cleared
    async fn clear_ignored_commands(&self) -> u32 {
        self.fingerprint.clear_ignored_commands()
    }

    /// Correct an ignored command - removes from ignored list and adds as pattern
    /// Returns true if the command was found and corrected
    async fn correct_ignored_command(&self, heard: String, meant: String) -> bool {
        println!("Correcting ignored command: '{}' -> '{}'", heard, meant);
        self.fingerprint.correct_ignored_command(&heard, &meant)
    }

    /// Manually add a command to the ignored list (for testing/debugging)
    async fn add_ignored_command(&self, heard: String, context: String) {
        self.fingerprint.add_ignored_command(&heard, &context)
    }

    // ========== Phase 13: Wyoming Protocol ==========

    /// Get current STT backend ("vosk" or "wyoming")
    async fn get_stt_backend(&self) -> String {
        crate::config_loader::SETTINGS.read()
            .map(|s| s.stt_backend.clone())
            .unwrap_or_else(|_| "vosk".to_string())
    }

    /// Get Wyoming server info (host, port, model)
    async fn get_wyoming_info(&self) -> (String, u16, String, bool) {
        let settings = crate::config_loader::SETTINGS.read().unwrap();
        (
            settings.wyoming_host.clone(),
            settings.wyoming_port,
            settings.wyoming_model.clone(),
            settings.wyoming_auto_start,
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let engine = Arc::new(Mutex::new(AudioEngine::new()));
    let cortex = Cortex::new();
    let ear = Arc::new(Mutex::new(Ear::new()));
    let fingerprint = Fingerprint::new();

    let _conn = Builder::session()?
        .name("org.speech.Service")?
        .serve_at("/org/speech/Service", SpeechService { 
            engine: engine.clone(), 
            cortex: cortex.clone(), 
            ear: ear.clone(),
            fingerprint: fingerprint.clone(),
        })?
        .build()
        .await?;

    println!("Speech Service running at org.speech.Service");

    // Start SSIP Shim
    let ssip_engine = engine.clone();
    tokio::spawn(async move {
        ssip::start_server(ssip_engine).await;
    });

    // Start Autonomous Mode (Wake Word + Command Processing)
    let config = config_loader::SETTINGS.read().unwrap();
    if config.enable_wake_word {
         let ear_handler = ear.clone();
         let engine_handler = engine.clone();
         let cortex_handler = cortex.clone();
         tokio::task::spawn_blocking(move || {
             if let Ok(ear_guard) = ear_handler.lock() {
                 ear_guard.start_autonomous_mode(engine_handler, cortex_handler);
             }
         });
    }

    pending::<()>().await;

    Ok(())
}
