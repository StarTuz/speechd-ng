pub mod espeak;



/// Trait that all speech synthesis backends must implement.
/// This allows us to plug in different engines (eSpeak, Piper, Coqui, etc.)
pub trait SpeechBackend: Send + Sync {
    /// Returns the data (stdout) of the synthesis process or an error
    fn synthesize(&self, text: &str) -> std::io::Result<Vec<u8>>;
    
    /// Returns the unique ID of the backend (e.g., "espeak-ng")
    fn id(&self) -> &'static str;
}
