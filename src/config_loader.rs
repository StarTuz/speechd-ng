use serde::Deserialize;
use config::{Config, File};
use std::sync::RwLock;
use lazy_static::lazy_static;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub ollama_url: String,
    pub ollama_model: String,
    pub memory_size: usize,
    pub enable_audio: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3".to_string(),
            memory_size: 50,
            enable_audio: true,
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
            .set_default("memory_size", 50)?
            .set_default("enable_audio", true)?
            // Merge with local config file (if exists)
            .add_source(File::with_name("Speech").required(false))
            // Merge with environment variables (e.g. SPEECH_OLLAMA_URL)
            .add_source(config::Environment::with_prefix("SPEECH"));

        builder.build()?.try_deserialize()
    }
}
