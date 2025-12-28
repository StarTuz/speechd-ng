use serde::Deserialize;
use config::{Config, File};
use std::sync::RwLock;
use lazy_static::lazy_static;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub ollama_url: String,
    pub ollama_model: String,
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3".to_string(),
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
        }
    }
}

lazy_static! {
    pub static ref SETTINGS: RwLock<Settings> = RwLock::new(
        Settings::new().expect("Failed to load settings")
    );
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let builder = Config::builder()
            // Connect to defaults
            .set_default("ollama_url", "http://localhost:11434")?
            .set_default("ollama_model", "llama3")?
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
            // Merge with local config file (if exists)
            .add_source(File::with_name("Speech").required(false))
            .add_source(File::with_name(&format!("{}/.config/speechd-ng/Speech", std::env::var("HOME").unwrap_or_default())).required(false))
            // Merge with environment variables (e.g. SPEECH_OLLAMA_URL)
            .add_source(config::Environment::with_prefix("SPEECH"));

        builder.build()?.try_deserialize()
    }
}
