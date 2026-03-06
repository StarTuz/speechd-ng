use super::{SpeechBackend, Voice};
use serde_json::Value;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct PiperBackend {
    binary_path: String,
    models_dir: PathBuf,
}

impl PiperBackend {
    #[cfg(test)]
    fn new_with_dir(models_dir: PathBuf) -> Self {
        Self {
            binary_path: "piper-tts".to_string(),
            models_dir,
        }
    }

    pub fn new() -> Self {
        let models_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".local/share/piper/models");

        let binary_path = crate::config_loader::SETTINGS
            .read()
            .map(|s| s.piper_binary.clone())
            .unwrap_or_else(|_| "piper-tts".to_string());

        Self {
            binary_path,
            models_dir,
        }
    }

    fn find_model_files(&self, voice_id: &str) -> Option<(PathBuf, PathBuf)> {
        let onnx = self.models_dir.join(format!("{}.onnx", voice_id));
        let config = self.models_dir.join(format!("{}.onnx.json", voice_id));

        if onnx.exists() && config.exists() {
            Some((onnx, config))
        } else {
            None
        }
    }

    fn parse_voice_metadata(&self, config_path: &PathBuf, voice_id: &str) -> Voice {
        let mut voice = Voice {
            id: voice_id.to_string(),
            name: voice_id.replace("_", " "),
            language: "unknown".to_string(),
            gender: "unknown".to_string(),
        };

        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                // Real Piper .onnx.json files have this:
                if let Some(quality) = json
                    .get("audio")
                    .and_then(|a| a.get("quality"))
                    .and_then(|q| q.as_str())
                {
                    voice.name = format!("{} ({})", voice_id.replace("_", " "), quality);
                }

                // Try to extract language from espeak.voice if present
                if let Some(espeak_voice) = json
                    .get("espeak")
                    .and_then(|e| e.get("voice"))
                    .and_then(|v| v.as_str())
                {
                    voice.language = espeak_voice.to_string();
                }
            }
        }

        voice
    }
}

impl SpeechBackend for PiperBackend {
    fn list_voices(&self) -> Result<Vec<Voice>> {
        let mut voices = Vec::new();

        if self.models_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.models_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("onnx") {
                        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let config_path = path.with_extension("onnx.json");
                            if config_path.exists() {
                                voices.push(self.parse_voice_metadata(&config_path, file_stem));
                            } else {
                                voices.push(Voice {
                                    id: file_stem.to_string(),
                                    name: file_stem.replace("_", " "),
                                    language: "unknown".to_string(),
                                    gender: "unknown".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(voices)
    }

    fn list_downloadable_voices(&self) -> Result<Vec<Voice>> {
        let url = "https://huggingface.co/rhasspy/piper-voices/raw/main/voices.json";
        let resp = reqwest::blocking::get(url).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Failed to fetch voices.json: {}", e),
            )
        })?;

        let json: Value = resp.json().map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Failed to parse voices.json: {}", e),
            )
        })?;

        let mut available = Vec::new();
        if let Some(obj) = json.as_object() {
            for (key, val) in obj {
                let lang = val
                    .get("language")
                    .and_then(|l| l.get("name_english"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown");
                let quality = val
                    .get("quality")
                    .and_then(|q| q.as_str())
                    .unwrap_or("unknown");
                let name = val.get("name").and_then(|n| n.as_str()).unwrap_or(key);

                available.push(Voice {
                    id: key.clone(),
                    name: format!("{} ({})", name, quality),
                    language: lang.to_string(),
                    gender: "unknown".to_string(),
                });
            }
        }

        // Sort by language then name
        available.sort_by(|a, b| a.language.cmp(&b.language).then(a.name.cmp(&b.name)));

        Ok(available)
    }

    fn download_voice(&self, voice_id: &str) -> Result<()> {
        let url = "https://huggingface.co/rhasspy/piper-voices/raw/main/voices.json";
        let resp = reqwest::blocking::get(url).map_err(|e| Error::new(ErrorKind::Other, e))?;
        let json: Value = resp
            .json()
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        let voice_info = json.get(voice_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("Voice {} not found in catalog", voice_id),
            )
        })?;

        let files = voice_info
            .get("files")
            .and_then(|f| f.as_object())
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No files found for voice"))?;

        if !self.models_dir.exists() {
            std::fs::create_dir_all(&self.models_dir)?;
        }

        for (path, _meta) in files {
            if path.ends_with(".onnx") || path.ends_with(".onnx.json") {
                let download_url = format!(
                    "https://huggingface.co/rhasspy/piper-voices/resolve/main/{}",
                    path
                );
                let mut resp = reqwest::blocking::get(download_url)
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;

                let filename = Path::new(path).file_name().ok_or_else(|| {
                    Error::new(ErrorKind::InvalidData, "Invalid filename in voices.json")
                })?;

                let dest_path = self.models_dir.join(filename);
                let mut file = std::fs::File::create(dest_path)?;
                std::io::copy(&mut resp, &mut file)?;
            }
        }

        Ok(())
    }

    fn synthesize(&self, text: &str, voice: Option<&str>) -> Result<Vec<u8>> {
        // Use first available model as default if none specified
        let voice_id = voice.unwrap_or("en_US-lessac-medium");

        let (onnx_path, _config_path) = self.find_model_files(voice_id)
            .or_else(|| {
                // Fall back to first available model
                let fallback = std::fs::read_dir(&self.models_dir).ok()?.flatten()
                    .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("onnx"))
                    .and_then(|e| {
                        let onnx = e.path();
                        let json = onnx.with_extension("onnx.json");
                        if json.exists() { Some((onnx, json)) } else { None }
                    })?;
                eprintln!(
                    "piper-tts: voice '{}' not found, falling back to '{}'",
                    voice_id,
                    fallback.0.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown")
                );
                Some(fallback)
            })
            .ok_or_else(|| Error::new(ErrorKind::NotFound,
                format!("No piper-tts voice model found in {:?}. Download one with: speechd-control download-voice <name>", self.models_dir)))?;

        let mut child = Command::new(&self.binary_path)
            .arg("-m")
            .arg(&onnx_path)
            .arg("-f")
            .arg("/dev/stdout") // piper-tts writes WAV to file path; /dev/stdout gives us stdout
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Write text to stdin and close it
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())?;
            stdin.write_all(b"\n")?;
            // stdin is dropped here, closing the pipe
        }

        // Now wait for completion and read output
        let output = child.wait_with_output()?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(Error::new(
                ErrorKind::Other,
                format!("Piper error: {}", err),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_model(dir: &std::path::Path, voice_id: &str, json: Option<&str>) {
        fs::write(dir.join(format!("{}.onnx", voice_id)), b"fake onnx").unwrap();
        if let Some(content) = json {
            fs::write(dir.join(format!("{}.onnx.json", voice_id)), content).unwrap();
        }
    }

    #[test]
    fn test_find_model_files_exact_match() {
        let dir = tempdir().unwrap();
        make_model(dir.path(), "en_US-lessac-medium", Some("{}"));
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let result = backend.find_model_files("en_US-lessac-medium");
        assert!(result.is_some());
        let (onnx, json) = result.unwrap();
        assert!(onnx.exists());
        assert!(json.exists());
    }

    #[test]
    fn test_find_model_files_missing_json_returns_none() {
        let dir = tempdir().unwrap();
        // Only .onnx, no .onnx.json
        fs::write(dir.path().join("en_US-lessac-medium.onnx"), b"fake onnx").unwrap();
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        assert!(backend.find_model_files("en_US-lessac-medium").is_none());
    }

    #[test]
    fn test_find_model_files_unknown_voice_returns_none() {
        let dir = tempdir().unwrap();
        make_model(dir.path(), "en_US-lessac-medium", Some("{}"));
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        assert!(backend.find_model_files("en_GB-semaine-medium").is_none());
    }

    #[test]
    fn test_synthesize_no_models_returns_not_found() {
        let dir = tempdir().unwrap();
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let err = backend.synthesize("hello", Some("en_US-lessac-medium")).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(err.to_string().contains("speechd-control download-voice"));
    }

    #[test]
    fn test_synthesize_empty_dir_not_found() {
        let dir = tempdir().unwrap();
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let err = backend.synthesize("hello", None).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotFound);
    }

    #[test]
    fn test_list_voices_empty_dir() {
        let dir = tempdir().unwrap();
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let voices = backend.list_voices().unwrap();
        assert!(voices.is_empty());
    }

    #[test]
    fn test_list_voices_nonexistent_dir() {
        let backend = PiperBackend::new_with_dir(PathBuf::from("/tmp/no-such-dir-speechd-ng-test"));
        let voices = backend.list_voices().unwrap();
        assert!(voices.is_empty());
    }

    #[test]
    fn test_list_voices_with_complete_model() {
        let dir = tempdir().unwrap();
        make_model(dir.path(), "en_US-lessac-medium", Some("{}"));
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let voices = backend.list_voices().unwrap();
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].id, "en_US-lessac-medium");
    }

    #[test]
    fn test_list_voices_onnx_without_json_still_listed() {
        let dir = tempdir().unwrap();
        // .onnx exists but no .json — still appears in list_voices with defaults
        fs::write(dir.path().join("en_US-ryan-low.onnx"), b"fake").unwrap();
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let voices = backend.list_voices().unwrap();
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].id, "en_US-ryan-low");
        assert_eq!(voices[0].language, "unknown");
    }

    #[test]
    fn test_parse_voice_metadata_extracts_language_and_quality() {
        let dir = tempdir().unwrap();
        let json = r#"{"audio": {"quality": "medium"}, "espeak": {"voice": "en-us"}}"#;
        make_model(dir.path(), "en_US-lessac-medium", Some(json));
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let config_path = dir.path().join("en_US-lessac-medium.onnx.json");
        let voice = backend.parse_voice_metadata(&config_path, "en_US-lessac-medium");
        assert_eq!(voice.language, "en-us");
        assert!(voice.name.contains("medium"));
    }

    #[test]
    fn test_parse_voice_metadata_missing_fields_uses_defaults() {
        let dir = tempdir().unwrap();
        make_model(dir.path(), "en_US-lessac-medium", Some("{}"));
        let backend = PiperBackend::new_with_dir(dir.path().to_path_buf());
        let config_path = dir.path().join("en_US-lessac-medium.onnx.json");
        let voice = backend.parse_voice_metadata(&config_path, "en_US-lessac-medium");
        assert_eq!(voice.language, "unknown");
        assert_eq!(voice.id, "en_US-lessac-medium");
    }
}
