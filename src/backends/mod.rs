pub mod espeak;
pub mod piper;

/// Represents a text-to-speech voice
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub language: String,
}

/// Trait that all speech synthesis backends must implement.
/// This allows us to plug in different engines (eSpeak, Piper, Coqui, etc.)
pub trait SpeechBackend: Send + Sync {
    /// Returns the data (stdout) of the synthesis process or an error
    /// 'voice' is an optional specific voice ID to use
    fn synthesize(&self, text: &str, voice: Option<&str>) -> std::io::Result<Vec<u8>>;
    
    /// Returns a list of supported voices installed locally
    fn list_voices(&self) -> std::io::Result<Vec<Voice>>;

    /// Returns a list of voices available for download (optional)
    fn list_downloadable_voices(&self) -> std::io::Result<Vec<Voice>> {
        Ok(Vec::new())
    }

    /// Downloads a voice given its ID (optional)
    fn download_voice(&self, _voice_id: &str) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Downloading not supported for this backend"))
    }
}
