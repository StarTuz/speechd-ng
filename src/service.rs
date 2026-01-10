use crate::config_loader;
use crate::cortex::Cortex;
use crate::ear::Ear;
use crate::engine::AudioOutput;
use crate::fingerprint::Fingerprint;
use crate::rate_limiter::{LimitType, RateLimiter};
use crate::security::SecurityAgent;
use serde_json::json;
use std::sync::{Arc, Mutex};
use zbus::{interface, message::Header, Connection};

pub struct SpeechService {
    pub engine: Arc<dyn AudioOutput + Send + Sync>,
    pub cortex: Cortex,
    pub ear: Arc<Mutex<Ear>>,
    pub fingerprint: Fingerprint,
    pub conn: Connection,
    pub rate_limiter: Arc<RateLimiter>,
    pub model_override: Arc<Mutex<Option<String>>>,
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
    async fn speak(
        &self,
        #[zbus(header)] header: Header<'_>,
        text: String,
    ) -> zbus::fdo::Result<()> {
        // Rate limit check
        if let Some(sender) = header.sender() {
            if !self.rate_limiter.check(sender.as_str(), LimitType::Tts) {
                println!("Rate limited: TTS for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
        }

        println!("Received speak request: {}", text);

        let audio_enabled = config_loader::SETTINGS
            .read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);

        if audio_enabled {
            self.engine.speak(&text, None);
        }
        let ai_enabled = config_loader::SETTINGS
            .read()
            .map(|s| s.enable_ai)
            .unwrap_or(true);
        if ai_enabled {
            self.cortex.observe(text).await;
        }
        Ok(())
    }

    #[zbus(name = "SpeakVoice")]
    async fn speak_voice(
        &self,
        #[zbus(header)] header: Header<'_>,
        text: String,
        voice: String,
    ) -> zbus::fdo::Result<()> {
        // Rate limit check
        if let Some(sender) = header.sender() {
            if !self.rate_limiter.check(sender.as_str(), LimitType::Tts) {
                println!("Rate limited: TTS for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
        }

        println!("Received speak request (voice: {}): {}", voice, text);

        let audio_enabled = config_loader::SETTINGS
            .read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);

        if audio_enabled {
            self.engine.speak(&text, Some(voice));
        }
        let ai_enabled = config_loader::SETTINGS
            .read()
            .map(|s| s.enable_ai)
            .unwrap_or(true);
        if ai_enabled {
            self.cortex.observe(text).await;
        }
        Ok(())
    }

    #[zbus(name = "ListVoices")]
    async fn list_voices(&self) -> Vec<(String, String)> {
        let list = self.engine.list_voices().await;
        list.into_iter().map(|v| (v.id, v.name)).collect()
    }

    #[zbus(name = "ListDownloadableVoices")]
    async fn list_downloadable_voices(&self) -> Vec<(String, String)> {
        let list = self.engine.list_downloadable_voices().await;
        list.into_iter()
            .map(|v| (v.id, format!("{} [{}]", v.name, v.language)))
            .collect()
    }

    #[zbus(name = "DownloadVoice")]
    async fn download_voice(
        &self,
        #[zbus(header)] header: Header<'_>,
        voice_id: String,
    ) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) =
                    SecurityAgent::check_permission_polkit(pid, "org.speech.service.manage").await
                {
                    return Err(zbus::fdo::Error::AccessDenied(format!(
                        "Polkit denied: {}",
                        e
                    )));
                }
            }
        }

        match self.engine.download_voice(voice_id).await {
            Ok(_) => Ok("Success".to_string()),
            Err(e) => Err(zbus::fdo::Error::Failed(format!("Error: {}", e))),
        }
    }

    #[zbus(name = "Think")]
    async fn think(
        &self,
        #[zbus(header)] header: Header<'_>,
        query: String,
    ) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) =
                    SecurityAgent::check_permission_polkit(pid, "org.speech.service.think").await
                {
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

        let ai_enabled = config_loader::SETTINGS
            .read()
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
                if let Err(e) =
                    SecurityAgent::check_permission_polkit(pid, "org.speech.service.listen").await
                {
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
        })
        .await;

        match result {
            Ok(s) => Ok(s),
            Err(e) => Ok(format!("Error joining audio task: {}", e)),
        }
    }

    #[zbus(name = "ListenVad")]
    async fn listen_vad(&self, #[zbus(header)] header: Header<'_>) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) =
                    SecurityAgent::check_permission_polkit(pid, "org.speech.service.listen").await
                {
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
        })
        .await;

        match result {
            Ok(s) => Ok(s),
            Err(e) => Ok(format!("Error joining audio task: {}", e)),
        }
    }

    #[zbus(name = "AddCorrection")]
    async fn add_correction(&self, heard: String, meant: String) -> zbus::fdo::Result<bool> {
        println!("Adding manual correction: '{}' -> '{}'", heard, meant);
        Ok(self.fingerprint.add_manual_correction(heard, meant))
    }

    #[zbus(name = "RollbackLastCorrection")]
    async fn rollback_last_correction(&self) -> zbus::fdo::Result<bool> {
        Ok(self.fingerprint.rollback_last_correction())
    }

    #[zbus(name = "TrainWord")]
    async fn train_word(
        &self,
        #[zbus(header)] header: Header<'_>,
        expected: String,
        duration_secs: u32,
    ) -> zbus::fdo::Result<(String, bool)> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) =
                    SecurityAgent::check_permission_polkit(pid, "org.speech.service.train").await
                {
                    eprintln!("Access Denied for TrainWord: {}", e);
                    return Err(zbus::fdo::Error::AccessDenied("Polkit denied".into()));
                }
            }
        }

        println!(
            "Training word '{}' for {} seconds...",
            expected, duration_secs
        );

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
                let success =
                    fingerprint.add_manual_correction(heard_trimmed.clone(), expected_clone);
                (heard_trimmed, success)
            } else {
                ("Error: Ear locked".to_string(), false)
            }
        })
        .await;

        match result {
            Ok((heard, success)) => {
                // Audio feedback on success
                if success {
                    let feedback =
                        format!("I heard {}. I'll remember that means {}.", heard, expected);
                    self.engine.speak(&feedback, None);
                }
                Ok((heard, success))
            }
            Err(e) => Err(zbus::fdo::Error::Failed(format!("Error: {}", e))),
        }
    }

    #[zbus(name = "GetFingerprintStats")]
    async fn get_fingerprint_stats(&self) -> (u32, u32, u32) {
        self.fingerprint.get_stats()
    }

    #[zbus(name = "ListPatterns")]
    async fn list_patterns(&self) -> zbus::fdo::Result<Vec<(String, String, String)>> {
        let patterns = self
            .fingerprint
            .get_all_patterns()
            .into_iter()
            .map(|(heard, meant, conf, source)| {
                (heard, meant, format!("{:.0}% ({})", conf * 100.0, source))
            })
            .collect();
        Ok(patterns)
    }

    #[zbus(name = "ExportFingerprint")]
    async fn export_fingerprint(&self, path: String) -> zbus::fdo::Result<bool> {
        println!("Exporting fingerprint to: {}", path);
        Ok(self.fingerprint.export_to_path(&path))
    }

    #[zbus(name = "ImportFingerprint")]
    async fn import_fingerprint(&self, path: String, merge: bool) -> zbus::fdo::Result<u32> {
        println!("Importing fingerprint from: {} (merge={})", path, merge);
        Ok(self.fingerprint.import_from_path(&path, merge))
    }

    #[zbus(name = "GetFingerprintPath")]
    async fn get_fingerprint_path(&self) -> String {
        self.fingerprint.get_path()
    }

    #[zbus(name = "GetIgnoredCommands")]
    async fn get_ignored_commands(&self) -> zbus::fdo::Result<Vec<(String, String, String)>> {
        Ok(self.fingerprint.get_ignored_commands())
    }

    #[zbus(name = "ClearIgnoredCommands")]
    async fn clear_ignored_commands(&self) -> zbus::fdo::Result<u32> {
        Ok(self.fingerprint.clear_ignored_commands())
    }

    #[zbus(name = "CorrectIgnoredCommand")]
    async fn correct_ignored_command(
        &self,
        heard: String,
        meant: String,
    ) -> zbus::fdo::Result<bool> {
        println!("Correcting ignored command: '{}' -> '{}'", heard, meant);
        Ok(self.fingerprint.correct_ignored_command(&heard, &meant))
    }

    #[zbus(name = "AddIgnoredCommand")]
    async fn add_ignored_command(&self, heard: String, context: String) {
        self.fingerprint.add_ignored_command(&heard, &context)
    }

    #[zbus(name = "GetSttBackend")]
    async fn get_stt_backend(&self) -> zbus::fdo::Result<String> {
        Ok(config_loader::SETTINGS.read().unwrap().stt_backend.clone())
    }

    #[zbus(name = "CheckWyomingHealth")]
    async fn check_wyoming_health(&self) -> (bool, String) {
        let (host, port) = {
            let settings = crate::config_loader::SETTINGS.read().unwrap();
            (settings.wyoming_host.clone(), settings.wyoming_port)
        };

        let addr = format!("{}:{}", host, port);
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => (
                true,
                format!("Successfully connected to Wyoming at {}", addr),
            ),
            Err(e) => (
                false,
                format!("Failed to connect to Wyoming at {}: {}", addr, e),
            ),
        }
    }

    #[zbus(name = "SetWakeWord")]
    async fn set_wake_word(
        &self,
        #[zbus(header)] header: Header<'_>,
        word: String,
    ) -> zbus::fdo::Result<bool> {
        if word.is_empty() {
            return Err(zbus::fdo::Error::Failed("Wake word cannot be empty".into()));
        }

        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) =
                    SecurityAgent::check_permission_polkit(pid, "org.speech.service.manage").await
                {
                    return Err(zbus::fdo::Error::AccessDenied(format!(
                        "Polkit denied: {}",
                        e
                    )));
                }
            }
        }

        println!("Setting wake word to: {}", word);
        {
            let mut settings = config_loader::SETTINGS.write().unwrap();
            settings.wake_word = word;
        }

        if let Ok(ear) = self.ear.lock() {
            ear.trigger_restart();
            Ok(true)
        } else {
            Err(zbus::fdo::Error::Failed("Ear locked".into()))
        }
    }

    #[zbus(name = "GetStatus")]
    async fn get_status(&self) -> zbus::fdo::Result<(bool, f32, String, u32, bool)> {
        let (ai, thresh, stt, rag) = {
            let s = config_loader::SETTINGS.read().unwrap();
            (
                s.enable_ai,
                s.passive_confidence_threshold,
                s.stt_backend.clone(),
                s.enable_rag,
            )
        };

        let (m, p, _) = self.fingerprint.get_stats();
        Ok((ai, thresh, stt, m + p, rag))
    }

    #[zbus(name = "GetWyomingInfo")]
    async fn get_wyoming_info(&self) -> zbus::fdo::Result<(String, u16, String, bool, String)> {
        let settings = crate::config_loader::SETTINGS.read().unwrap();
        Ok((
            settings.wyoming_host.clone(),
            settings.wyoming_port,
            settings.wyoming_model.clone(),
            settings.wyoming_auto_start,
            settings.wyoming_device.clone(),
        ))
    }

    #[zbus(name = "PlayAudio")]
    async fn play_audio(
        &self,
        #[zbus(header)] header: Header<'_>,
        url: String,
    ) -> zbus::fdo::Result<String> {
        // Rate limit check
        if let Some(sender) = header.sender() {
            if !self.rate_limiter.check(sender.as_str(), LimitType::Audio) {
                println!("Rate limited: Audio for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
        }

        println!("Received PlayAudio request for URL: {}", url);

        match self.engine.play_audio(&url).await {
            Ok(()) => Ok(String::new()),
            Err(e) => Ok(e),
        }
    }

    #[zbus(name = "StopAudio")]
    async fn stop_audio(&self) -> bool {
        println!("Received StopAudio request");

        self.engine.stop_audio().await
    }

    #[zbus(name = "SetVolume")]
    async fn set_volume(&self, volume: f64) -> bool {
        println!("Received SetVolume request: {}", volume);

        self.engine.set_volume(volume as f32).await
    }

    #[zbus(name = "GetVolume")]
    async fn get_volume(&self) -> f64 {
        let settings = crate::config_loader::SETTINGS.read().unwrap();
        settings.playback_volume as f64
    }

    #[zbus(name = "GetPlaybackStatus")]
    async fn get_playback_status(&self) -> (bool, String) {
        self.engine.get_playback_status().await
    }

    #[zbus(name = "SpeakChannel")]
    async fn speak_channel(&self, text: String, voice: String, channel: String) -> bool {
        println!(
            "Received SpeakChannel: '{}' -> {} (channel: {})",
            text, voice, channel
        );

        let audio_enabled = config_loader::SETTINGS
            .read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);

        if audio_enabled {
            let voice_opt = if voice.is_empty() { None } else { Some(voice) };
            self.engine.speak_channel(&text, voice_opt, &channel);
            return true;
        }
        false
    }

    #[zbus(name = "PlayAudioChannel")]
    async fn play_audio_channel(&self, url: String, channel: String) -> String {
        println!("Received PlayAudioChannel: {} -> {}", url, channel);

        match self.engine.play_audio_channel(&url, &channel).await {
            Ok(()) => String::new(),
            Err(e) => e,
        }
    }

    #[zbus(name = "ListChannels")]
    async fn list_channels(&self) -> Vec<(String, String)> {
        vec![
            ("left".to_string(), "Left speaker/ear only".to_string()),
            ("right".to_string(), "Right speaker/ear only".to_string()),
            ("center".to_string(), "Both at 70% (mono-like)".to_string()),
            ("stereo".to_string(), "Full stereo (default)".to_string()),
        ]
    }

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

                if stdout.is_empty() {
                    eprintln!("ListSinks: wpctl returned empty stdout, stderr: {}", stderr);
                }

                let mut sinks = Vec::new();
                let mut in_sinks_section = false;

                for line in stdout.lines() {
                    if line.contains("Sinks:") && !line.contains("Sources:") {
                        in_sinks_section = true;
                        continue;
                    }
                    if in_sinks_section {
                        if line.contains("Sources:")
                            || line.contains("Streams:")
                            || line.contains("Filters:")
                        {
                            break;
                        }

                        if !line.contains("[vol:") && !line.contains(".") {
                            continue;
                        }

                        let is_default = line.contains("*");
                        let cleaned: String = line
                            .chars()
                            .filter(|c| !['│', '├', '└', '─', '┬', '┤', '┴', '┼'].contains(c))
                            .collect();
                        let trimmed = cleaned.trim().trim_start_matches('*').trim();

                        if let Some(dot_pos) = trimmed.find('.') {
                            if let Ok(id) = trimmed[..dot_pos].trim().parse::<u32>() {
                                let rest = trimmed[dot_pos + 1..].trim();
                                let name = if let Some(vol_pos) = rest.find("[vol:") {
                                    rest[..vol_pos].trim()
                                } else {
                                    rest
                                };

                                if !name.is_empty() {
                                    sinks.push((
                                        id,
                                        name.to_string(),
                                        name.to_string(),
                                        is_default,
                                    ));
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

    #[zbus(name = "SpeakToDevice")]
    async fn speak_to_device(&self, text: String, voice: String, device_id: u32) -> bool {
        println!("Received SpeakToDevice: '{}' -> device {}", text, device_id);

        let current_default = std::process::Command::new("/usr/bin/wpctl")
            .args(["inspect", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .ok()
            .and_then(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout
                    .lines()
                    .find(|l| l.trim().starts_with("id"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|s| s.trim_matches(',').parse::<u32>().ok())
            });

        let set_result = std::process::Command::new("/usr/bin/wpctl")
            .args(["set-default", &device_id.to_string()])
            .status();

        if set_result.is_err() || !set_result.unwrap().success() {
            eprintln!("Failed to set default sink to {}", device_id);
            return false;
        }

        let audio_enabled = config_loader::SETTINGS
            .read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);

        if audio_enabled {
            let voice_opt = if voice.is_empty() { None } else { Some(voice) };
            self.engine.speak(&text, voice_opt);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if let Some(prev_id) = current_default {
            let _ = std::process::Command::new("/usr/bin/wpctl")
                .args(["set-default", &prev_id.to_string()])
                .status();
        }

        true
    }

    #[zbus(name = "GetDefaultSink")]
    async fn get_default_sink(&self) -> (u32, String) {
        let sinks = self.list_sinks().await;
        sinks
            .into_iter()
            .find(|(_, _, _, is_default)| *is_default)
            .map(|(id, name, _, _)| (id, name))
            .unwrap_or((0, String::new()))
    }

    #[zbus(name = "GetBrainStatus")]
    async fn get_brain_status(&self) -> (bool, String, Vec<String>) {
        let model = self
            .model_override
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                crate::config_loader::SETTINGS
                    .read()
                    .map(|s| s.ollama_model.clone())
                    .unwrap_or_else(|_| "unknown".to_string())
            });

        let url = crate::config_loader::SETTINGS
            .read()
            .map(|s| s.ollama_url.clone())
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap_or_default();

        let res = client.get(&format!("{}/api/tags", url)).send().await;

        let mut available = Vec::new();
        let is_running = match res {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(models) = json["models"].as_array() {
                        for m in models {
                            if let Some(name) = m["name"].as_str() {
                                available.push(name.to_string());
                            }
                        }
                    }
                }
                true
            }
            Err(_) => false,
        };

        (is_running, model, available)
    }

    #[zbus(name = "ManageBrain")]
    async fn manage_brain(&self, action: String, param: String) -> bool {
        match action.as_str() {
            "start" => {
                let _ = std::process::Command::new("systemctl")
                    .args(["start", "ollama"])
                    .status();
                std::process::Command::new("systemctl")
                    .args(["--user", "start", "ollama"])
                    .status()
                    .is_ok()
            }
            "stop" => {
                let _ = std::process::Command::new("systemctl")
                    .args(["stop", "ollama"])
                    .status();
                std::process::Command::new("systemctl")
                    .args(["--user", "stop", "ollama"])
                    .status()
                    .is_ok()
            }
            "pull" => {
                let url = {
                    let settings = crate::config_loader::SETTINGS.read().unwrap();
                    settings.ollama_url.clone()
                };
                let client = reqwest::Client::new();
                let res = client
                    .post(&format!("{}/api/pull", url))
                    .json(&json!({"name": param, "stream": false}))
                    .send()
                    .await;
                res.is_ok()
            }
            "use" => self.set_brain_model(param).await,
            _ => false,
        }
    }

    #[zbus(name = "SetBrainModel")]
    async fn set_brain_model(&self, model: String) -> bool {
        if model.is_empty() {
            return false;
        }

        if let Ok(mut override_lock) = self.model_override.lock() {
            let old = override_lock
                .clone()
                .unwrap_or_else(|| "<default>".to_string());
            *override_lock = Some(model.clone());
            println!("Switched AI model: {} → {}", old, model);
            true
        } else {
            println!("SetBrainModel: Failed to acquire lock");
            false
        }
    }

    #[zbus(name = "DescribeScreen")]
    async fn describe_screen(
        &self,
        #[zbus(header)] header: Header<'_>,
        prompt: String,
    ) -> zbus::fdo::Result<String> {
        // Polkit authorization check
        if let Some(sender) = header.sender() {
            if let Ok(pid) = SecurityAgent::get_sender_pid(&self.conn, sender.as_str()).await {
                if let Err(e) =
                    SecurityAgent::check_permission_polkit(pid, "org.speech.service.think").await
                {
                    eprintln!("Access Denied: {}", e);
                    return Err(zbus::fdo::Error::AccessDenied("Polkit denied".into()));
                }
            }
            // Rate limit check
            if !self.rate_limiter.check(sender.as_str(), LimitType::Ai) {
                println!("Rate limited: AI (Vision) for sender {}", sender);
                return Err(zbus::fdo::Error::Failed("Rate limited".into()));
            }
        }

        let ai_enabled = config_loader::SETTINGS
            .read()
            .map(|s| s.enable_ai)
            .unwrap_or(true);

        if !ai_enabled {
            return Ok("AI disabled".to_string());
        }

        println!("Received screen description request: {}", prompt);
        let description = self.cortex.query_local_vision(prompt).await;
        Ok(description)
    }
}
