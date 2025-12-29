mod engine;
mod cortex;
mod config_loader;
mod security;
mod backends;
mod ear;
mod ssip;
mod fingerprint;
mod rate_limiter;
use engine::AudioEngine;
use cortex::Cortex;
use ear::Ear;
use fingerprint::Fingerprint;
use security::SecurityAgent;
use rate_limiter::{RateLimiter, LimitType};
use std::error::Error;
use std::future::pending;
use std::sync::{Arc, Mutex};
use zbus::{interface, connection::Builder, message::Header, Connection};

struct SpeechService {
    engine: Arc<Mutex<AudioEngine>>,
    cortex: Cortex,
    ear: Arc<Mutex<Ear>>,
    fingerprint: Fingerprint,
    conn: Connection,
    rate_limiter: Arc<RateLimiter>,
}

#[interface(name = "org.speech.Service")]
impl SpeechService {
    #[zbus(name = "Ping")]
    async fn ping(&self) -> String {
        "pong".to_string()
    }

    #[zbus(name = "GetVersion")]
    async fn get_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[zbus(name = "Speak")]
    async fn speak(&self, #[zbus(header)] header: Header<'_>, text: String) -> zbus::fdo::Result<()> {
        // Rate limit check
        if let Some(sender) = header.sender() {
            if !self.rate_limiter.check(sender.as_str(), LimitType::Tts) {
                println!("Rate limited: TTS for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
        }
        
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
        Ok(())
    }

    #[zbus(name = "SpeakVoice")]
    async fn speak_voice(&self, #[zbus(header)] header: Header<'_>, text: String, voice: String) -> zbus::fdo::Result<()> {
         // Rate limit check
         if let Some(sender) = header.sender() {
             if !self.rate_limiter.check(sender.as_str(), LimitType::Tts) {
                 println!("Rate limited: TTS for sender {}", sender);
                 return Err(zbus::fdo::Error::Failed("Rate limited".into()));
             }
         }
         
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
         Ok(())
    }

    #[zbus(name = "ListVoices")]
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

    #[zbus(name = "ListDownloadableVoices")]
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

    #[zbus(name = "DownloadVoice")]
    async fn download_voice(&self, #[zbus(header)] header: Header<'_>, voice_id: String) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) = SecurityAgent::check_permission_polkit(pid, "org.speech.service.manage").await {
                    return Err(zbus::fdo::Error::AccessDenied(format!("Polkit denied: {}", e)));
                }
            }
        }

        let engine = if let Ok(engine) = self.engine.lock() {
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = engine {
            match engine.download_voice(voice_id).await {
                Ok(_) => Ok("Success".to_string()),
                Err(e) => Err(zbus::fdo::Error::Failed(format!("Error: {}", e))),
            }
        } else {
             Err(zbus::fdo::Error::Failed("Engine locked".to_string()))
        }
    }

    #[zbus(name = "Think")]
    async fn think(&self, #[zbus(header)] header: Header<'_>, query: String) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) = SecurityAgent::check_permission_polkit(pid, "org.speech.service.think").await {
                    eprintln!("Access Denied: {}", e);
                    return Err(zbus::fdo::Error::AccessDenied("Polkit denied".into()));
                }
            }
            // Rate limit check
            if !self.rate_limiter.check(sender.as_str(), LimitType::Ai) {
                println!("Rate limited: AI for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
        }

        let ai_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_ai)
            .unwrap_or(true);

        if !ai_enabled {
            return Ok("AI disabled".to_string());
        }

        println!("Received thought query: {}", query);
        let response = self.cortex.query(query).await;
        Ok(response)
    }

    #[zbus(name = "Listen")]
    async fn listen(&self, #[zbus(header)] header: Header<'_>) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) = SecurityAgent::check_permission_polkit(pid, "org.speech.service.listen").await {
                    eprintln!("Access Denied: {}", e);
                    return Err(zbus::fdo::Error::AccessDenied("Polkit denied".into()));
                }
            }
            // Rate limit check
            if !self.rate_limiter.check(sender.as_str(), LimitType::Listen) {
                println!("Rate limited: Listen for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
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
            Ok(s) => Ok(s),
            Err(e) => Ok(format!("Error joining audio task: {}", e)),
        }
    }

    /// Listen with Voice Activity Detection (Phase 12)
    /// Waits for speech, records until silence, then transcribes
    #[zbus(name = "ListenVad")]
    async fn listen_vad(&self, #[zbus(header)] header: Header<'_>) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) = SecurityAgent::check_permission_polkit(pid, "org.speech.service.listen").await {
                    eprintln!("Access Denied: {}", e);
                    return Err(zbus::fdo::Error::AccessDenied("Polkit denied".into()));
                }
            }
            // Rate limit check
            if !self.rate_limiter.check(sender.as_str(), LimitType::Listen) {
                println!("Rate limited: Listen for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
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
            Ok(s) => Ok(s),
            Err(e) => Ok(format!("Error joining audio task: {}", e)),
        }
    }

    // ========== Phase 9: Voice Training API ==========

    /// Add a manual voice correction (heard -> meant)
    /// This is used when the user knows what ASR mishears
    #[zbus(name = "AddCorrection")]
    async fn add_correction(&self, heard: String, meant: String) -> zbus::fdo::Result<bool> {
        println!("Adding manual correction: '{}' -> '{}'", heard, meant);
        Ok(self.fingerprint.add_manual_correction(heard, meant))
    }

    /// Undo the last correction (manual or passive)
    #[zbus(name = "RollbackLastCorrection")]
    async fn rollback_last_correction(&self) -> zbus::fdo::Result<bool> {
        Ok(self.fingerprint.rollback_last_correction())
    }

    /// Train a word by recording user speech and learning what ASR hears
    /// Returns (what_asr_heard, success)
    #[zbus(name = "TrainWord")]
    async fn train_word(&self, #[zbus(header)] header: Header<'_>, expected: String, duration_secs: u32) -> zbus::fdo::Result<(String, bool)> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) = SecurityAgent::check_permission_polkit(pid, "org.speech.service.train").await {
                    eprintln!("Access Denied for TrainWord: {}", e);
                    return Err(zbus::fdo::Error::AccessDenied("Polkit denied".into()));
                }
            }
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
                Ok((heard, success))
            },
            Err(e) => Err(zbus::fdo::Error::Failed(format!("Error: {}", e))),
        }
    }

    /// Get fingerprint statistics (manual_patterns, passive_patterns, command_count)
    #[zbus(name = "GetFingerprintStats")]
    async fn get_fingerprint_stats(&self) -> (u32, u32, u32) {
        self.fingerprint.get_stats()
    }

    /// List all learned patterns (for debugging/UI)
    /// List all learned patterns (for debugging/UI)
    #[zbus(name = "ListPatterns")]
    async fn list_patterns(&self) -> zbus::fdo::Result<Vec<(String, String, String)>> {
        let patterns = self.fingerprint.get_all_patterns()
            .into_iter()
            .map(|(heard, meant, conf, source)| {
                (heard, meant, format!("{:.0}% ({})", conf * 100.0, source))
            })
            .collect();
        Ok(patterns)
    }

    // ========== Phase 10: Pattern Import/Export ==========

    /// Export fingerprint to a file
    /// Export fingerprint to a file
    /// Returns true if successful
    async fn export_fingerprint(&self, path: String) -> zbus::fdo::Result<bool> {
        println!("Exporting fingerprint to: {}", path);
        Ok(self.fingerprint.export_to_path(&path))
    }

    /// Import fingerprint from a file
    /// If merge=true, adds new patterns without overwriting existing
    /// If merge=false, replaces current fingerprint entirely
    /// Returns total pattern count after import
    async fn import_fingerprint(&self, path: String, merge: bool) -> zbus::fdo::Result<u32> {
        println!("Importing fingerprint from: {} (merge={})", path, merge);
        Ok(self.fingerprint.import_from_path(&path, merge))
    }

    /// Get the path to the fingerprint data file
    async fn get_fingerprint_path(&self) -> String {
        self.fingerprint.get_path()
    }

    // ========== Phase 11: Ignored Commands Tracking ==========

    /// Get all ignored commands (heard, timestamp, context)
    async fn get_ignored_commands(&self) -> zbus::fdo::Result<Vec<(String, String, String)>> {
        Ok(self.fingerprint.get_ignored_commands())
    }

    /// Clear all ignored commands
    /// Returns count of commands cleared
    async fn clear_ignored_commands(&self) -> zbus::fdo::Result<u32> {
        Ok(self.fingerprint.clear_ignored_commands())
    }

    /// Correct an ignored command - removes from ignored list and adds as pattern
    /// Returns true if the command was found and corrected
    async fn correct_ignored_command(&self, heard: String, meant: String) -> zbus::fdo::Result<bool> {
        println!("Correcting ignored command: '{}' -> '{}'", heard, meant);
        Ok(self.fingerprint.correct_ignored_command(&heard, &meant))
    }

    /// Manually add a command to the ignored list (for testing/debugging)
    async fn add_ignored_command(&self, heard: String, context: String) {
        self.fingerprint.add_ignored_command(&heard, &context)
    }

    // ========== Phase 13: Wyoming Protocol ==========

    /// Get current STT backend ("vosk" or "wyoming")
    /// Returns diagnostic status: (ai_enabled, passive_threshold, stt_backend, total_patterns)
    async fn get_status(&self) -> zbus::fdo::Result<(bool, f32, String, u32)> {
        let (ai, thresh, stt) = {
            let s = config_loader::SETTINGS.read().unwrap();
            (s.enable_ai, s.passive_confidence_threshold, s.stt_backend.clone())
        };
        
        let (m, p, _) = self.fingerprint.get_stats();
        Ok((ai, thresh, stt, m + p))
    }

    /// Returns Wyoming connection info: (host, port, model, auto_start)
    async fn get_wyoming_info(&self) -> zbus::fdo::Result<(String, u16, String, bool)> {
        let settings = crate::config_loader::SETTINGS.read().unwrap();
        Ok((
            settings.wyoming_host.clone(),
            settings.wyoming_port,
            settings.wyoming_model.clone(),
            settings.wyoming_auto_start,
        ))
    }
    
    // ========== Phase 15: Streaming Media Player ==========
    
    /// Play audio from a URL
    /// Returns empty string on success, error message on failure
    #[zbus(name = "PlayAudio")]
    async fn play_audio(&self, #[zbus(header)] header: Header<'_>, url: String) -> zbus::fdo::Result<String> {
        // Rate limit check
        if let Some(sender) = header.sender() {
            if !self.rate_limiter.check(sender.as_str(), LimitType::Audio) {
                println!("Rate limited: Audio for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
        }
        
        println!("Received PlayAudio request for URL: {}", url);
        
        let engine = if let Ok(engine) = self.engine.lock() {
            Some(engine.clone())
        } else {
            return Err(zbus::fdo::Error::Failed("Engine locked".into()));
        };
        
        if let Some(engine) = engine {
            match engine.play_audio(&url).await {
                Ok(()) => Ok(String::new()),  // Empty string = success
                Err(e) => Ok(e), // Legacy: return error as string for now if not rate limited
            }
        } else {
             Err(zbus::fdo::Error::Failed("No engine".into()))
        }
    }
    
    /// Stop current audio playback
    /// Returns true if something was stopped
    #[zbus(name = "StopAudio")]
    async fn stop_audio(&self) -> bool {
        println!("Received StopAudio request");
        
        let engine = if let Ok(engine) = self.engine.lock() {
            Some(engine.clone())
        } else {
            return false;
        };
        
        if let Some(engine) = engine {
            engine.stop_audio().await
        } else {
            false
        }
    }
    
    /// Set playback volume (0.0 - 1.0)
    /// Returns true on success
    #[zbus(name = "SetVolume")]
    async fn set_volume(&self, volume: f64) -> bool {
        println!("Received SetVolume request: {}", volume);
        
        let engine = if let Ok(engine) = self.engine.lock() {
            Some(engine.clone())
        } else {
            return false;
        };
        
        if let Some(engine) = engine {
            engine.set_volume(volume as f32).await
        } else {
            false
        }
    }
    
    /// Get current volume setting (0.0 - 1.0)
    #[zbus(name = "GetVolume")]
    async fn get_volume(&self) -> f64 {
        let settings = crate::config_loader::SETTINGS.read().unwrap();
        settings.playback_volume as f64
    }
    
    /// Get playback status
    /// Returns (is_playing, current_url_or_empty)
    #[zbus(name = "GetPlaybackStatus")]
    async fn get_playback_status(&self) -> (bool, String) {
        let engine = if let Ok(engine) = self.engine.lock() {
            Some(engine.clone())
        } else {
            return (false, String::new());
        };
        
        if let Some(engine) = engine {
            engine.get_playback_status().await
        } else {
            (false, String::new())
        }
    }
    
    // ========== Phase 16: Multi-Channel Audio ==========
    
    /// Speak text to a specific audio channel
    /// channel: "left", "right", "center", or "stereo" (default)
    /// Returns true on success
    #[zbus(name = "SpeakChannel")]
    async fn speak_channel(&self, text: String, voice: String, channel: String) -> bool {
        println!("Received SpeakChannel: '{}' -> {} (channel: {})", text, voice, channel);
        
        let audio_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);
        
        if audio_enabled {
            if let Ok(engine) = self.engine.lock() {
                let voice_opt = if voice.is_empty() { None } else { Some(voice) };
                engine.speak_channel(&text, voice_opt, &channel);
                return true;
            }
        }
        false
    }
    
    /// Play audio from URL to a specific channel
    /// channel: "left", "right", "center", or "stereo"
    /// Returns empty string on success, error message on failure
    #[zbus(name = "PlayAudioChannel")]
    async fn play_audio_channel(&self, url: String, channel: String) -> String {
        println!("Received PlayAudioChannel: {} -> {}", url, channel);
        
        let engine = if let Ok(engine) = self.engine.lock() {
            Some(engine.clone())
        } else {
            return "Error: Engine locked".to_string();
        };
        
        if let Some(engine) = engine {
            match engine.play_audio_channel(&url, &channel).await {
                Ok(()) => String::new(),
                Err(e) => e,
            }
        } else {
            "Error: No engine".to_string()
        }
    }
    
    /// List available audio channels
    /// Returns list of (channel_name, description) tuples
    #[zbus(name = "ListChannels")]
    async fn list_channels(&self) -> Vec<(String, String)> {
        vec![
            ("left".to_string(), "Left speaker/ear only".to_string()),
            ("right".to_string(), "Right speaker/ear only".to_string()),
            ("center".to_string(), "Both at 70% (mono-like)".to_string()),
            ("stereo".to_string(), "Full stereo (default)".to_string()),
        ]
    }
    
    // ========== Phase 16b: PipeWire Device Routing ==========
    
    /// List available PipeWire audio sinks
    /// Returns list of (id, name, description, is_default) tuples
    #[zbus(name = "ListSinks")]
    async fn list_sinks(&self) -> Vec<(u32, String, String, bool)> {
        // Parse wpctl status output to get sinks
        match std::process::Command::new("/usr/bin/wpctl")
            .arg("status")
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                // Debug logging
                if stdout.is_empty() {
                    eprintln!("ListSinks: wpctl returned empty stdout, stderr: {}", stderr);
                }
                
                let mut sinks = Vec::new();
                let mut in_sinks_section = false;
                
                for line in stdout.lines() {
                    // Check for Sinks section start (with box-drawing chars)
                    if line.contains("Sinks:") && !line.contains("Sources:") {
                        in_sinks_section = true;
                        continue;
                    }
                    if in_sinks_section {
                        // End of sinks section - Sources line or empty sink line
                        if line.contains("Sources:") || line.contains("Streams:") || 
                           line.contains("Filters:") {
                            break;
                        }
                        
                        // Skip empty or header lines
                        if !line.contains("[vol:") && !line.contains(".") {
                            continue;
                        }
                        
                        // Parse sink line: " │  *   68. SB Omni Surround 5.1 [vol: 1.00]"
                        let is_default = line.contains("*");
                        
                        // Strip box-drawing characters and whitespace
                        let cleaned: String = line.chars()
                            .filter(|c| !['│', '├', '└', '─', '┬', '┤', '┴', '┼'].contains(c))
                            .collect();
                        let trimmed = cleaned.trim().trim_start_matches('*').trim();
                        
                        // Find number before first dot
                        if let Some(dot_pos) = trimmed.find('.') {
                            if let Ok(id) = trimmed[..dot_pos].trim().parse::<u32>() {
                                let rest = trimmed[dot_pos + 1..].trim();
                                // Extract name before [vol:
                                let name = if let Some(vol_pos) = rest.find("[vol:") {
                                    rest[..vol_pos].trim()
                                } else {
                                    rest
                                };
                                
                                if !name.is_empty() {
                                    sinks.push((id, name.to_string(), name.to_string(), is_default));
                                }
                            }
                        }
                    }
                }
                sinks
            }
            Err(e) => {
                eprintln!("Failed to run wpctl: {}", e);
                Vec::new()
            }
        }
    }
    
    /// Speak text to a specific PipeWire device by ID
    /// First sets the device as default, speaks, then optionally restores
    /// Returns true on success
    #[zbus(name = "SpeakToDevice")]
    async fn speak_to_device(&self, text: String, voice: String, device_id: u32) -> bool {
        println!("Received SpeakToDevice: '{}' -> device {}", text, device_id);
        
        // Get current default sink to restore later
        let current_default = std::process::Command::new("/usr/bin/wpctl")
            .args(["inspect", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .ok()
            .and_then(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                // Extract id from output
                stdout.lines()
                    .find(|l| l.trim().starts_with("id"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|s| s.trim_matches(',').parse::<u32>().ok())
            });
        
        // Set target device as default
        let set_result = std::process::Command::new("/usr/bin/wpctl")
            .args(["set-default", &device_id.to_string()])
            .status();
        
        if set_result.is_err() || !set_result.unwrap().success() {
            eprintln!("Failed to set default sink to {}", device_id);
            return false;
        }
        
        // Speak
        let audio_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);
        
        if audio_enabled {
            if let Ok(engine) = self.engine.lock() {
                let voice_opt = if voice.is_empty() { None } else { Some(voice) };
                // Use blocking speak to ensure audio completes before restoring default
                engine.speak(&text, voice_opt);
            }
        }
        
        // Wait a moment for audio to start playing through the new device
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Restore previous default if we had one
        if let Some(prev_id) = current_default {
            let _ = std::process::Command::new("/usr/bin/wpctl")
                .args(["set-default", &prev_id.to_string()])
                .status();
        }
        
        true
    }
    
    /// Get the current default audio sink
    /// Returns (id, name) or (0, "") if not found
    #[zbus(name = "GetDefaultSink")]
    async fn get_default_sink(&self) -> (u32, String) {
        // Find the default sink from ListSinks
        let sinks = self.list_sinks().await;
        sinks.into_iter()
            .find(|(_, _, _, is_default)| *is_default)
            .map(|(id, name, _, _)| (id, name))
            .unwrap_or((0, String::new()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let engine = Arc::new(Mutex::new(AudioEngine::new()));
    let cortex = Cortex::new();
    let ear = Arc::new(Mutex::new(Ear::new()));
    let fingerprint = Fingerprint::new();

    // Build the D-Bus connection first so we can clone it for SpeechService
    let conn = Builder::session()?
        .name("org.speech.Service")?
        .build()
        .await?;
    
    // Initialize rate limiter from config
    let config = config_loader::SETTINGS.read().unwrap();
    println!("Loaded Rate Limits - TTS: {}, AI: {}, Audio: {}, Listen: {}", 
        config.rate_limit_tts, config.rate_limit_ai, config.rate_limit_audio, config.rate_limit_listen);
        
    let rate_limiter = Arc::new(RateLimiter::new(
        config.rate_limit_tts,
        config.rate_limit_ai,
        config.rate_limit_audio,
        config.rate_limit_listen,
    ));
    drop(config); // Release the read lock
    
    // Register the service interface with a clone of the connection
    conn.object_server().at("/org/speech/Service", SpeechService { 
        engine: engine.clone(), 
        cortex: cortex.clone(), 
        ear: ear.clone(),
        fingerprint: fingerprint.clone(),
        conn: conn.clone(),
        rate_limiter: rate_limiter.clone(),
    }).await?;

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
