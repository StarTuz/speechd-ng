use crate::config_loader;
use crate::cortex::Cortex;
use crate::ear::Ear;
use crate::engine::AudioOutput;
use crate::fingerprint::Fingerprint;
use crate::rate_limiter::{LimitType, RateLimiter};
use crate::service_helpers::{audio_devices, brain, guards};
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
    // ─── Utility ───

    #[zbus(name = "Ping")]
    async fn ping(&self) -> String {
        "pong".to_string()
    }

    #[zbus(name = "GetVersion")]
    async fn get_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[zbus(name = "Reload")]
    async fn reload(&self) -> zbus::fdo::Result<String> {
        match config_loader::Settings::new() {
            Ok(new_settings) => {
                let new_tts = new_settings.rate_limit_tts;
                let new_ai = new_settings.rate_limit_ai;
                let new_audio = new_settings.rate_limit_audio;
                let new_listen = new_settings.rate_limit_listen;
                match config_loader::try_write_settings(|s| *s = new_settings) {
                    Ok(_) => {
                        self.rate_limiter
                            .update_limits(new_tts, new_ai, new_audio, new_listen);
                        println!("Config reloaded");
                        Ok("Config reloaded".to_string())
                    }
                    Err(e) => Err(zbus::fdo::Error::Failed(e.to_string())),
                }
            }
            Err(e) => Err(zbus::fdo::Error::Failed(format!("Invalid config: {}", e))),
        }
    }

    // ─── TTS ───

    #[zbus(name = "Speak")]
    async fn speak(
        &self,
        #[zbus(header)] header: Header<'_>,
        text: String,
    ) -> zbus::fdo::Result<()> {
        guards::check_rate_limit(&header, &self.rate_limiter, LimitType::Tts, "TTS")?;
        println!("Received speak request: {}", text);

        if config_loader::read_settings(|s| s.enable_audio, true) {
            self.engine.speak(&text, None);
        }
        if config_loader::read_settings(|s| s.enable_ai, true) {
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
        guards::check_rate_limit(&header, &self.rate_limiter, LimitType::Tts, "TTS")?;
        println!("Received speak request (voice: {}): {}", voice, text);

        if config_loader::read_settings(|s| s.enable_audio, true) {
            self.engine.speak(&text, Some(voice));
        }
        if config_loader::read_settings(|s| s.enable_ai, true) {
            self.cortex.observe(text).await;
        }
        Ok(())
    }

    #[zbus(name = "ListVoices")]
    async fn list_voices(&self) -> Vec<(String, String)> {
        self.engine
            .list_voices()
            .await
            .into_iter()
            .map(|v| (v.id, v.name))
            .collect()
    }

    #[zbus(name = "ListDownloadableVoices")]
    async fn list_downloadable_voices(&self) -> Vec<(String, String)> {
        self.engine
            .list_downloadable_voices()
            .await
            .into_iter()
            .map(|v| (v.id, format!("{} [{}]", v.name, v.language)))
            .collect()
    }

    #[zbus(name = "DownloadVoice")]
    async fn download_voice(
        &self,
        #[zbus(header)] header: Header<'_>,
        voice_id: String,
    ) -> zbus::fdo::Result<String> {
        guards::check_polkit(&self.conn, &header, "org.speech.service.manage").await?;
        match self.engine.download_voice(voice_id).await {
            Ok(_) => Ok("Success".to_string()),
            Err(e) => Err(zbus::fdo::Error::Failed(format!("Error: {}", e))),
        }
    }

    #[zbus(name = "SpeakChannel")]
    async fn speak_channel(&self, text: String, voice: String, channel: String) -> bool {
        println!(
            "Received SpeakChannel: '{}' -> {} (channel: {})",
            text, voice, channel
        );
        if config_loader::read_settings(|s| s.enable_audio, true) {
            let voice_opt = if voice.is_empty() { None } else { Some(voice) };
            self.engine.speak_channel(&text, voice_opt, &channel);
            return true;
        }
        false
    }

    // ─── AI ───

    #[zbus(name = "Think")]
    async fn think(
        &self,
        #[zbus(header)] header: Header<'_>,
        query: String,
    ) -> zbus::fdo::Result<String> {
        guards::check_polkit(&self.conn, &header, "org.speech.service.think").await?;
        guards::check_rate_limit(&header, &self.rate_limiter, LimitType::Ai, "AI")?;

        if !config_loader::read_settings(|s| s.enable_ai, true) {
            return Ok("AI disabled".to_string());
        }

        println!("Received thought query: {}", query);
        Ok(self.cortex.query(query).await)
    }

    #[zbus(name = "DescribeScreen")]
    async fn describe_screen(
        &self,
        #[zbus(header)] header: Header<'_>,
        prompt: String,
    ) -> zbus::fdo::Result<String> {
        guards::check_polkit(&self.conn, &header, "org.speech.service.think").await?;
        guards::check_rate_limit(&header, &self.rate_limiter, LimitType::Ai, "AI (Vision)")?;

        if !config_loader::read_settings(|s| s.enable_ai, true) {
            return Ok("AI disabled".to_string());
        }

        println!("Received screen description request: {}", prompt);
        Ok(self.cortex.query_local_vision(prompt).await)
    }

    #[zbus(name = "GetBrainStatus")]
    async fn get_brain_status(&self) -> (bool, String, Vec<String>) {
        brain::get_brain_status(&self.model_override).await
    }

    #[zbus(name = "ManageBrain")]
    async fn manage_brain(&self, action: String, param: String) -> bool {
        brain::manage_brain(&action, &param, &self.model_override).await
    }

    #[zbus(name = "SetBrainModel")]
    async fn set_brain_model(&self, model: String) -> bool {
        brain::set_brain_model(&model, &self.model_override)
    }

    // ─── STT / Listen ───

    #[zbus(name = "Listen")]
    async fn listen(&self, #[zbus(header)] header: Header<'_>) -> zbus::fdo::Result<String> {
        guards::check_polkit(&self.conn, &header, "org.speech.service.listen").await?;
        guards::check_rate_limit(&header, &self.rate_limiter, LimitType::Listen, "Listen")?;
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
        guards::check_polkit(&self.conn, &header, "org.speech.service.listen").await?;
        guards::check_rate_limit(&header, &self.rate_limiter, LimitType::Listen, "Listen")?;
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

    #[zbus(name = "GetSttBackend")]
    async fn get_stt_backend(&self) -> zbus::fdo::Result<String> {
        config_loader::try_read_settings(|s| s.stt_backend.clone())
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    #[zbus(name = "CheckWyomingHealth")]
    async fn check_wyoming_health(&self) -> (bool, String) {
        let (host, port) = config_loader::read_settings(
            |s| (s.wyoming_host.clone(), s.wyoming_port),
            ("127.0.0.1".to_string(), 10301),
        );
        let addr = format!("{}:{}", host, port);
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => (true, format!("Successfully connected to Wyoming at {}", addr)),
            Err(e) => (false, format!("Failed to connect to Wyoming at {}: {}", addr, e)),
        }
    }

    #[zbus(name = "GetWyomingInfo")]
    async fn get_wyoming_info(&self) -> zbus::fdo::Result<(String, u16, String, bool, String)> {
        config_loader::try_read_settings(|s| {
            (
                s.wyoming_host.clone(),
                s.wyoming_port,
                s.wyoming_model.clone(),
                s.wyoming_auto_start,
                s.wyoming_device.clone(),
            )
        })
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    // ─── Fingerprint / Voice Learning ───

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
        guards::check_polkit(&self.conn, &header, "org.speech.service.train").await?;
        println!("Training word '{}' for {} seconds...", expected, duration_secs);

        let ear = self.ear.clone();
        let fingerprint = self.fingerprint.clone();
        let expected_clone = expected.clone();

        let result = tokio::task::spawn_blocking(move || {
            if let Ok(ear_guard) = ear.lock() {
                let heard = ear_guard.record_and_transcribe(duration_secs as u64);
                let heard_trimmed = heard.trim().to_string();
                if heard_trimmed.is_empty() {
                    return ("[no speech detected]".to_string(), false);
                }
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
        Ok(self
            .fingerprint
            .get_all_patterns()
            .into_iter()
            .map(|(heard, meant, conf, source)| {
                (heard, meant, format!("{:.0}% ({})", conf * 100.0, source))
            })
            .collect())
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

    // ─── Configuration / Settings ───

    #[zbus(name = "SetWakeWord")]
    async fn set_wake_word(
        &self,
        #[zbus(header)] header: Header<'_>,
        word: String,
    ) -> zbus::fdo::Result<bool> {
        if word.is_empty() {
            return Err(zbus::fdo::Error::Failed("Wake word cannot be empty".into()));
        }
        guards::check_polkit(&self.conn, &header, "org.speech.service.manage").await?;

        println!("Setting wake word to: {}", word);
        if let Err(e) = config_loader::try_write_settings(|s| s.wake_word = word) {
            return Err(zbus::fdo::Error::Failed(e.to_string()));
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
        let (ai, thresh, stt, rag) = config_loader::try_read_settings(|s| {
            (s.enable_ai, s.passive_confidence_threshold, s.stt_backend.clone(), s.enable_rag)
        })
        .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let (m, p, _) = self.fingerprint.get_stats();
        Ok((ai, thresh, stt, m + p, rag))
    }

    #[zbus(name = "GetVolume")]
    async fn get_volume(&self) -> f64 {
        config_loader::read_settings(|s| s.playback_volume as f64, 1.0)
    }

    #[zbus(name = "SetVolume")]
    async fn set_volume(&self, volume: f64) -> bool {
        println!("Received SetVolume request: {}", volume);
        self.engine.set_volume(volume as f32).await
    }

    // ─── Audio Playback ───

    #[zbus(name = "PlayAudio")]
    async fn play_audio(
        &self,
        #[zbus(header)] header: Header<'_>,
        url: String,
    ) -> zbus::fdo::Result<String> {
        guards::check_rate_limit(&header, &self.rate_limiter, LimitType::Audio, "Audio")?;
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

    #[zbus(name = "GetPlaybackStatus")]
    async fn get_playback_status(&self) -> (bool, String) {
        self.engine.get_playback_status().await
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

    // ─── Audio Devices ───

    #[zbus(name = "ListSinks")]
    async fn list_sinks(&self) -> Vec<(u32, String, String, bool)> {
        audio_devices::list_sinks()
    }

    #[zbus(name = "SpeakToDevice")]
    async fn speak_to_device(&self, text: String, voice: String, device_id: u32) -> bool {
        println!("Received SpeakToDevice: '{}' -> device {}", text, device_id);

        let current_default = audio_devices::get_current_default_sink_id();
        if !audio_devices::set_default_sink(device_id) {
            eprintln!("Failed to set default sink to {}", device_id);
            return false;
        }

        if config_loader::read_settings(|s| s.enable_audio, true) {
            let voice_opt = if voice.is_empty() { None } else { Some(voice) };
            self.engine.speak(&text, voice_opt);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if let Some(prev_id) = current_default {
            audio_devices::set_default_sink(prev_id);
        }

        true
    }

    #[zbus(name = "GetDefaultSink")]
    async fn get_default_sink(&self) -> (u32, String) {
        audio_devices::list_sinks()
            .into_iter()
            .find(|(_, _, _, is_default)| *is_default)
            .map(|(id, name, _, _)| (id, name))
            .unwrap_or((0, String::new()))
    }
}
