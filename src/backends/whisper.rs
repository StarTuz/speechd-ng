//! Native Whisper backend using whisper.cpp via whisper-rs bindings
//!
//! This provides a pure Rust STT backend without Python dependencies.

use std::sync::{Arc, Mutex, OnceLock};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Global whisper context - model loaded once and kept in memory
static WHISPER_CTX: OnceLock<Arc<Mutex<WhisperContext>>> = OnceLock::new();

/// Native Whisper backend for speech-to-text
pub struct WhisperBackend {
    model_path: String,
    language: String,
}

impl WhisperBackend {
    /// Create a new Whisper backend instance
    pub fn new(model_path: &str, language: &str) -> Self {
        Self {
            model_path: model_path.to_string(),
            language: language.to_string(),
        }
    }

    /// Get or initialize the whisper context (lazy-loaded, stays in memory)
    fn get_or_init_context(&self) -> Result<Arc<Mutex<WhisperContext>>, String> {
        // Check if already initialized
        if let Some(ctx) = WHISPER_CTX.get() {
            return Ok(ctx.clone());
        }

        // Initialize new context
        // Expand ~ to home directory
        let expanded_path = if self.model_path.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            self.model_path.replacen("~", &home, 1)
        } else {
            self.model_path.clone()
        };

        println!("Whisper: Loading model from {}...", expanded_path);
        let params = WhisperContextParameters::default();

        let ctx = WhisperContext::new_with_params(&expanded_path, params)
            .map_err(|e| format!("Failed to load Whisper model: {:?}", e))?;

        println!("Whisper: Model loaded successfully");

        // Store in global and return
        let arc_ctx = Arc::new(Mutex::new(ctx));
        let _ = WHISPER_CTX.set(arc_ctx.clone());
        Ok(arc_ctx)
    }

    /// Transcribe audio from a WAV file
    pub fn transcribe(&self, wav_path: &str) -> Result<String, String> {
        let start = std::time::Instant::now();

        // Load and convert audio
        let audio_data = self.load_audio(wav_path)?;
        println!(
            "Whisper: Loaded {} samples from {}",
            audio_data.len(),
            wav_path
        );

        // Get or load the model
        let ctx = self.get_or_init_context()?;
        let ctx_guard = ctx.lock().map_err(|_| "Context lock error")?;

        // Create state for this transcription
        let mut state = ctx_guard
            .create_state()
            .map_err(|e| format!("Failed to create state: {:?}", e))?;

        // Configure parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Set language (empty string = auto-detect)
        if !self.language.is_empty() && self.language != "auto" {
            params.set_language(Some(&self.language));
        }

        // Disable printing to stdout
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Single segment mode for short audio
        params.set_single_segment(true);

        // Run inference - returns number of segments on success
        let _num_segments = state
            .full(params, &audio_data)
            .map_err(|e| format!("Transcription failed: {:?}", e))?;

        // Get number of segments (for logging)
        let _num_segments = state.full_n_segments();

        // Collect segments using as_iter()
        let mut text = String::new();
        for segment in state.as_iter() {
            if let Ok(segment_text) = segment.to_str() {
                text.push_str(segment_text);
                text.push(' ');
            }
        }

        let result = text.trim().to_string();
        println!("Whisper: Transcribed '{}' in {:?}", result, start.elapsed());

        Ok(result)
    }

    /// Load audio from WAV file and convert to f32 mono 16kHz
    fn load_audio(&self, wav_path: &str) -> Result<Vec<f32>, String> {
        let reader =
            hound::WavReader::open(wav_path).map_err(|e| format!("Failed to open WAV: {}", e))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels as usize;

        // Read samples based on format
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .filter_map(|s| s.ok())
                .collect(),
            hound::SampleFormat::Int => {
                let bit_depth = spec.bits_per_sample;
                reader
                    .into_samples::<i32>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / (1 << (bit_depth - 1)) as f32)
                    .collect()
            }
        };

        // Convert to mono if stereo
        let mono: Vec<f32> = if channels > 1 {
            samples
                .chunks(channels)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect()
        } else {
            samples
        };

        // Resample to 16kHz if needed (whisper.cpp expects 16kHz)
        if sample_rate != 16000 {
            Ok(self.resample(&mono, sample_rate, 16000))
        } else {
            Ok(mono)
        }
    }

    /// Simple linear interpolation resampling
    fn resample(&self, input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
        let ratio = from_rate as f64 / to_rate as f64;
        let output_len = (input.len() as f64 / ratio) as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_idx = i as f64 * ratio;
            let idx = src_idx as usize;
            let frac = src_idx - idx as f64;

            let sample = if idx + 1 < input.len() {
                input[idx] * (1.0 - frac as f32) + input[idx + 1] * frac as f32
            } else if idx < input.len() {
                input[idx]
            } else {
                0.0
            };

            output.push(sample);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample() {
        let backend = WhisperBackend::new("", "en");
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = backend.resample(&input, 8000, 16000);
        assert!(output.len() > input.len());
    }
}
