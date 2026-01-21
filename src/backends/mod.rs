use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;

// --- AI / Brain Backends (Phase 20) ---

#[async_trait]
pub trait BrainBackend: Send + Sync {
    async fn prompt(&self, system: &str, context: &str, user: &str) -> Result<String, String>;
    async fn stream(
        &self,
        system: &str,
        context: &str,
        user: &str,
        images: Option<Vec<String>>,
    ) -> Receiver<String>;
}

pub mod ollama;
pub mod openai;

// --- Audio / TTS Backends ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub gender: String,
}

pub trait SpeechBackend: Send + Sync {
    fn synthesize(&self, text: &str, voice: Option<&str>) -> std::io::Result<Vec<u8>>;
    fn list_voices(&self) -> std::io::Result<Vec<Voice>>;
    fn list_downloadable_voices(&self) -> std::io::Result<Vec<Voice>> {
        Ok(Vec::new())
    }
    fn download_voice(&self, _voice_id: &str) -> std::io::Result<()> {
        Ok(())
    }
}

pub mod espeak;
pub mod piper;
pub mod whisper;
