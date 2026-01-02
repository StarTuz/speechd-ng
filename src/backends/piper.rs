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
    pub fn new() -> Self {
        let models_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".local/share/piper/models");

        let binary_path = crate::config_loader::SETTINGS
            .read()
            .map(|s| s.piper_binary.clone())
            .unwrap_or_else(|_| "piper".to_string());

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
        let voice_id = voice.unwrap_or("en_US-lessac-medium");

        let (onnx_path, _config_path) = self.find_model_files(voice_id).ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!(
                    "Piper model not found locally for voice: {}. Please download it first.",
                    voice_id
                ),
            )
        })?;

        let mut child = Command::new(&self.binary_path)
            .arg("-m")
            .arg(&onnx_path)
            .arg("--output_file")
            .arg("-") // Output WAV to stdout
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
