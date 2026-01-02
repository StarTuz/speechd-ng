use config::{Config, File};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub ollama_url: String,
    pub ollama_model: String,
    pub enable_ai: bool,
    pub passive_confidence_threshold: f32,
    pub piper_model: String,
    pub piper_binary: String,
    pub tts_backend: String,
    pub memory_size: usize,
    pub enable_audio: bool,
    pub wake_word: String,
    pub enable_wake_word: bool,
    // VAD Settings (Phase 12)
    pub vad_speech_threshold: i16,
    pub vad_silence_threshold: i16,
    pub vad_silence_duration_ms: u64,
    pub vad_max_duration_ms: u64,
    // STT Settings
    pub stt_backend: String, // "vosk", "wyoming", or "whisper"
    pub wyoming_host: String,
    pub wyoming_port: u16,
    pub wyoming_auto_start: bool,
    pub wyoming_device: String, // "cpu" or "cuda"
    pub wyoming_model: String,  // "tiny", "base", "small", "medium", "large"
    // Native Whisper settings
    pub whisper_model_path: String, // Path to .bin model file
    pub whisper_language: String,   // "en", "auto", etc.
    // Media Player Settings (Phase 15)
    pub max_audio_size_mb: u64,     // Max audio file download size in MB
    pub playback_timeout_secs: u64, // Timeout for audio downloads
    pub playback_volume: f32,       // Default volume (0.0 - 1.0)
    // Rate Limiting Settings (Phase 17b)
    pub rate_limit_tts: u32,    // TTS requests per minute
    pub rate_limit_ai: u32,     // AI/Think requests per minute
    pub rate_limit_audio: u32,  // PlayAudio requests per minute
    pub rate_limit_listen: u32, // Listen requests per minute
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3".to_string(),
            enable_ai: true,
            passive_confidence_threshold: 0.1,
            piper_model: "en_US-lessac-medium".to_string(),
            piper_binary: "piper".to_string(),
            tts_backend: "espeak".to_string(),
            memory_size: 50,
            enable_audio: true,
            wake_word: "startuz".to_string(),
            enable_wake_word: false,
            // VAD defaults
            vad_speech_threshold: 500,
            vad_silence_threshold: 400,
            vad_silence_duration_ms: 1500,
            vad_max_duration_ms: 15000,
            // Wyoming defaults
            stt_backend: "vosk".to_string(),
            wyoming_host: "127.0.0.1".to_string(),
            wyoming_port: 10301,
            wyoming_auto_start: true,
            wyoming_device: "cpu".to_string(),
            wyoming_model: "tiny".to_string(),
            // Native Whisper defaults
            whisper_model_path: format!(
                "{}/.cache/whisper/ggml-tiny.en.bin",
                std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
            ),
            whisper_language: "en".to_string(),
            // Media Player defaults (Phase 15)
            max_audio_size_mb: 50,
            playback_timeout_secs: 30,
            playback_volume: 1.0,
            // Rate Limiting defaults (Phase 17b)
            rate_limit_tts: 30,
            rate_limit_ai: 10,
            rate_limit_audio: 20,
            rate_limit_listen: 30,
        }
    }
}

lazy_static! {
    pub static ref SETTINGS: RwLock<Settings> =
        RwLock::new(Settings::new().expect("Failed to load settings"));
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let builder = Config::builder()
            // Connect to defaults
            .set_default("ollama_url", "http://localhost:11434")?
            .set_default("ollama_model", "llama3")?
            .set_default("enable_ai", true)?
            .set_default("passive_confidence_threshold", 0.1)?
            .set_default("piper_model", "en_US-lessac-medium")?
            .set_default("piper_binary", "piper")?
            .set_default("tts_backend", "espeak")?
            .set_default("memory_size", 50)?
            .set_default("enable_audio", true)?
            .set_default("wake_word", "startuz")?
            .set_default("enable_wake_word", false)?
            // VAD defaults
            .set_default("vad_speech_threshold", 500)?
            .set_default("vad_silence_threshold", 400)?
            .set_default("vad_silence_duration_ms", 1500)?
            .set_default("vad_max_duration_ms", 15000)?
            // Wyoming defaults
            .set_default("stt_backend", "vosk")?
            .set_default("wyoming_host", "127.0.0.1")?
            .set_default("wyoming_port", 10301)?
            .set_default("wyoming_auto_start", true)?
            .set_default("wyoming_device", "cpu")?
            .set_default("wyoming_model", "tiny")?
            // Native Whisper defaults
            .set_default(
                "whisper_model_path",
                format!(
                    "{}/.cache/whisper/ggml-tiny.en.bin",
                    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
                ),
            )?
            .set_default("whisper_language", "en")?
            // Media Player defaults (Phase 15)
            .set_default("max_audio_size_mb", 50)?
            .set_default("playback_timeout_secs", 30)?
            .set_default("playback_volume", 1.0)?
            // Rate Limiting defaults (Phase 17b)
            .set_default("rate_limit_tts", 30)?
            .set_default("rate_limit_ai", 10)?
            .set_default("rate_limit_audio", 20)?
            .set_default("rate_limit_listen", 30)?
            // Merge with local config file (if exists)
            .add_source(File::with_name("Speech").required(false))
            .add_source(
                File::with_name(&format!(
                    "{}/.config/speechd-ng/Speech",
                    std::env::var("HOME").unwrap_or_default()
                ))
                .required(false),
            )
            // Merge with environment variables (e.g. SPEECH_OLLAMA_URL)
            .add_source(config::Environment::with_prefix("SPEECH"));

        let settings: Settings = builder.build()?.try_deserialize()?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn validate(&self) -> Result<(), config::ConfigError> {
        if self.playback_volume < 0.0 || self.playback_volume > 1.0 {
            return Err(config::ConfigError::Message(format!(
                "Invalid playback_volume: {}. Must be between 0.0 and 1.0",
                self.playback_volume
            )));
        }
        if self.memory_size == 0 {
            return Err(config::ConfigError::Message(
                "memory_size must be greater than 0".to_string(),
            ));
        }
        if self.vad_speech_threshold <= 0 {
            return Err(config::ConfigError::Message(
                "vad_speech_threshold must be positive".to_string(),
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_load() {
        let settings = Settings::new().expect("Failed to load settings");
        assert!(settings.memory_size > 0);
    }
}
