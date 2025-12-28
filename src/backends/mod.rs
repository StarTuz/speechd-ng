pub mod espeak;



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
    
    /// Returns the unique ID of the backend (e.g., "espeak-ng")
    fn id(&self) -> &'static str;

    /// Returns a list of supported voices
    fn list_voices(&self) -> std::io::Result<Vec<Voice>>;
}
