use std::env;
// Removed unused PathBuf
use std::process::Command;

pub struct VisionHelper;

impl VisionHelper {
    /// Captures the current screen content and returns it as a Base64 encoded JPEG/PNG.
    pub fn capture_screen() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let session_type = env::var("XDG_SESSION_TYPE")
            .unwrap_or_default()
            .to_uppercase();
        let desktop = env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_uppercase();
        let session_desktop = env::var("GDMSESSION").unwrap_or_default().to_uppercase();

        let raw_bytes = if session_type.contains("WAYLAND") {
            if desktop.contains("KDE") {
                match Self::capture_kde_wayland() {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        eprintln!("Vision: KDE capture failed: {}. Trying fallbacks...", e);
                        Self::capture_grim_generic()
                            .or_else(|_| Self::capture_x11())
                            .map_err(|last_e| {
                                format!(
                                    "KDE capture failed ({}) and fallbacks failed too ({})",
                                    e, last_e
                                )
                            })?
                    }
                }
            } else if desktop.contains("GNOME") || session_desktop.contains("GNOME") {
                Self::capture_gnome().or_else(|e| {
                    eprintln!("Vision: GNOME capture failed: {}. Trying fallbacks...", e);
                    Self::capture_grim_generic().or_else(|_| Self::capture_x11())
                })?
            } else if desktop.contains("SWAY") || desktop.contains("HYPRLAND") {
                Self::capture_wlroots().or_else(|e| {
                    eprintln!("Vision: wlroots capture failed: {}. Trying fallbacks...", e);
                    Self::capture_x11()
                })?
            } else {
                // Try generic Grim as fallback for other Wayland compositors
                Self::capture_grim_generic().or_else(|e| {
                    eprintln!(
                        "Vision: Generic Wayland capture failed: {}. Trying X11 fallback...",
                        e
                    );
                    Self::capture_x11()
                })?
            }
        } else {
            Self::capture_x11()?
        };

        Ok(raw_bytes)
    }

    fn capture_kde_wayland() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Try spectacle first
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let speechd_cache = cache_dir.join("speechd-ng");
        std::fs::create_dir_all(&speechd_cache)?;
        let output_path = speechd_cache.join("vision_capture.png");
        let output_path_str = output_path.to_string_lossy();

        let output = Command::new("spectacle")
            .args(&["-b", "-n", "-o", &output_path_str])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                if output_path.exists() {
                    let bytes = std::fs::read(&output_path)?;
                    let _ = std::fs::remove_file(&output_path); // Cleanup
                    Ok(bytes)
                } else {
                    Err(format!(
                        "Spectacle reported success but output file not found at {}",
                        output_path_str
                    )
                    .into())
                }
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                Err(format!(
                    "Spectacle failed with exit code {:?}: {}",
                    o.status.code(),
                    stderr
                )
                .into())
            }
            Err(e) => Err(format!("Failed to start spectacle: {}", e).into()),
        }
    }

    fn capture_gnome() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // gnome-screenshot -f /tmp/...
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let speechd_cache = cache_dir.join("speechd-ng");
        std::fs::create_dir_all(&speechd_cache)?;
        let output_path = speechd_cache.join("vision_capture_gnome.png");
        let output_path_str = output_path.to_string_lossy();

        let status = Command::new("gnome-screenshot")
            .args(&["-f", &output_path_str])
            .status();

        match status {
            Ok(s) if s.success() => {
                let bytes = std::fs::read(&output_path)?;
                let _ = std::fs::remove_file(&output_path);
                Ok(bytes)
            }
            _ => Err("GNOME screenshot failed".into()),
        }
    }

    fn capture_wlroots() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Self::capture_grim_generic()
    }

    fn capture_grim_generic() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // grim - (stdout)
        let output = Command::new("grim").arg("-").output()?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err("Grim capture failed".into())
        }
    }

    fn capture_x11() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Fallback for X11 environments

        // Try ImageMagick 'import'
        let output = Command::new("import")
            .args(&["-window", "root", "png:-"])
            .output();

        if let Ok(o) = output {
            if o.status.success() {
                return Ok(o.stdout);
            }
        }

        // Try scrot with temp file
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let speechd_cache = cache_dir.join("speechd-ng");
        std::fs::create_dir_all(&speechd_cache)?;
        let output_path = speechd_cache.join("vision_capture_x11.png");
        let output_path_str = output_path.to_string_lossy();

        let status = Command::new("scrot")
            .args(&["--overwrite", &output_path_str])
            .status();

        match status {
            Ok(s) if s.success() => {
                let bytes = std::fs::read(&output_path)?;
                let _ = std::fs::remove_file(&output_path);
                Ok(bytes)
            }
            _ => Err("X11 capture failed (tried import and scrot)".into()),
        }
    }
}

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::moondream::{Config, Model};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;

pub struct TheEye {
    model: Option<Model>,
    tokenizer: Option<Tokenizer>,
    device: Device,
}

impl TheEye {
    pub fn new() -> Self {
        // Initialize device (CUDA if available, else CPU)
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);

        TheEye {
            model: None,
            tokenizer: None,
            device,
        }
    }

    /// Loads the Moondream model from Hugging Face Hub.
    /// This is a heavy operation and should only be called when needed.
    pub fn load_model(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.model.is_some() {
            return Ok(());
        }

        println!("The Eye: Loading Moondream 2 model... (this may take a while first time)");

        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(
            "vikhyatk/moondream1".to_string(),
            RepoType::Model,
            "f6e9da68e8f1b78b8f3ee10905d56826db7a5802".to_string(),
        ));

        let model_file = repo.get("model.safetensors")?;
        let tokenizer_file = repo.get("tokenizer.json")?;

        let config = Config::v2();
        let tokenizer = Tokenizer::from_file(tokenizer_file).map_err(|e| e.to_string())?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_file], DType::F32, &self.device)?
        };
        let model = Model::new(&config, vb)?;

        self.model = Some(model);
        self.tokenizer = Some(tokenizer);

        println!("The Eye: Model loaded successfully.");
        Ok(())
    }

    /// Describes the given image bytes using the Moondream model.
    pub fn describe_image(
        &mut self,
        image_bytes: &[u8],
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Ensure model is loaded
        if self.model.is_none() {
            self.load_model()?;
        }

        // Preprocess image BEFORE mutable borrow of model (fixes E0502)
        let img_tensor = self.preprocess_image(image_bytes)?;

        let model = self.model.as_mut().unwrap();
        model.text_model.clear_kv_cache();
        let tokenizer = self.tokenizer.as_ref().unwrap();

        // Note: Generic normalization now handled in preprocess_image

        // Encode image
        let image_embeds = model.vision_encoder.forward(&img_tensor)?;

        // Prepare prompt tokens - Moondream standard template
        let formatted_prompt = format!("\n\nQuestion: {}\n\nAnswer:", prompt);
        let mut tokens = tokenizer
            .encode(formatted_prompt.as_str(), true)
            .map_err(|e| e.to_string())?
            .get_ids()
            .to_vec();
        if tokens.is_empty() {
            return Err("Empty prompt".into());
        }

        // Special tokens
        let special_token = tokenizer
            .get_vocab(true)
            .get("<|endoftext|>")
            .copied()
            .unwrap_or(0);
        let bos_token = special_token;
        let eos_token = special_token;

        let mut generated_text = String::new();
        let mut logits_processor =
            candle_transformers::generation::LogitsProcessor::new(1337, None, None);

        // Generation loop
        for index in 0..128 {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let ctxt = &tokens[tokens.len().saturating_sub(context_size)..];
            let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;

            let logits = if index > 0 {
                model.text_model.forward(&input)?
            } else {
                let bos_tensor = Tensor::new(&[bos_token], &self.device)?.unsqueeze(0)?;
                model
                    .text_model
                    .forward_with_img(&bos_tensor, &input, &image_embeds)?
            };

            let logits = logits.squeeze(0)?.to_dtype(DType::F32)?;

            // Apply repeat penalty
            let repeat_penalty = 1.2f32;
            let repeat_last_n = 64;
            let start_at = tokens.len().saturating_sub(repeat_last_n);
            let logits = candle_transformers::utils::apply_repeat_penalty(
                &logits,
                repeat_penalty,
                &tokens[start_at..],
            )?;

            let next_token = logits_processor.sample(&logits)?;
            tokens.push(next_token);

            if next_token == eos_token {
                break;
            }

            if let Ok(token_str) = tokenizer.decode(&[next_token], true) {
                generated_text.push_str(&token_str);
            }
        }

        Ok(generated_text.trim().to_string())
    }

    /// Refactored helper for image preprocessing to allow independent testing.
    fn preprocess_image(
        &self,
        image_bytes: &[u8],
    ) -> Result<Tensor, Box<dyn std::error::Error + Send + Sync>> {
        let img = image::load_from_memory(image_bytes)?;
        let img = img
            .resize_exact(378, 378, image::imageops::FilterType::Triangle)
            .to_rgb8();

        let img_tensor = Tensor::from_vec(img.into_raw(), (378, 378, 3), &self.device)?
            .permute((2, 0, 1))?
            .to_dtype(DType::F32)?
            .affine(1. / 255., 0.)?;

        let mean = Tensor::new(&[0.485f32, 0.456, 0.406], &self.device)?.reshape((3, 1, 1))?;
        let std = Tensor::new(&[0.229f32, 0.224, 0.225], &self.device)?.reshape((3, 1, 1))?;

        let img_tensor = img_tensor
            .broadcast_sub(&mean)?
            .broadcast_div(&std)?
            .unsqueeze(0)?;

        Ok(img_tensor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adversarial_image_shapes() {
        let vision = TheEye::new();

        // 1. Extreme aspect ratio (32:9 equivalent)
        let wide_img = image::DynamicImage::new_rgb8(3200, 900);
        let mut wide_bytes = std::io::Cursor::new(Vec::new());
        wide_img
            .write_to(&mut wide_bytes, image::ImageFormat::Png)
            .unwrap();
        let wide_bytes = wide_bytes.into_inner();

        let tensor = vision.preprocess_image(&wide_bytes).unwrap();
        assert_eq!(tensor.dims(), &[1, 3, 378, 378]);

        // 2. Zero-entropy image (solid color)
        let solid_img = image::RgbImage::new(100, 100); // Defaults to black
        let img = image::DynamicImage::ImageRgb8(solid_img);
        let mut solid_bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut solid_bytes, image::ImageFormat::Png)
            .unwrap();
        let solid_bytes = solid_bytes.into_inner();

        let tensor = vision.preprocess_image(&solid_bytes).unwrap();
        assert_eq!(tensor.dims(), &[1, 3, 378, 378]);

        // Ensure no NaNs in the normalized tensor
        let vals = tensor.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert!(vals.iter().all(|f: &f32| f.is_finite()));
    }
}
