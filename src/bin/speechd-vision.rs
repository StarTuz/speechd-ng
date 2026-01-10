//! SpeechD-Vision: Standalone Vision Service for SpeechD-NG
//!
//! This is an optional, separate service that provides screen capture and
//! image analysis using the Moondream 2 vision-language model.
//!
//! Communicates with the main speechd-ng daemon via D-Bus.

use std::env;
use std::process::Command;
use std::sync::{Arc, Mutex};

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::moondream::{Config, Model};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use zbus::{connection, interface, Connection};

/// Screen capture helper supporting multiple desktop environments
struct VisionHelper;

impl VisionHelper {
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
                Self::capture_kde_wayland()
                    .or_else(|_| Self::capture_grim_generic())
                    .or_else(|_| Self::capture_x11())?
            } else if desktop.contains("GNOME") || session_desktop.contains("GNOME") {
                Self::capture_gnome()
                    .or_else(|_| Self::capture_grim_generic())
                    .or_else(|_| Self::capture_x11())?
            } else if desktop.contains("SWAY") || desktop.contains("HYPRLAND") {
                Self::capture_wlroots().or_else(|_| Self::capture_x11())?
            } else {
                Self::capture_grim_generic().or_else(|_| Self::capture_x11())?
            }
        } else {
            Self::capture_x11()?
        };

        Ok(raw_bytes)
    }

    fn capture_kde_wayland() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let speechd_cache = cache_dir.join("speechd-vision");
        std::fs::create_dir_all(&speechd_cache)?;
        let output_path = speechd_cache.join("capture.png");
        let output_path_str = output_path.to_string_lossy();

        for attempt in 1..=5 {
            let output = Command::new("spectacle")
                .args(["-b", "-n", "-o", &output_path_str])
                .output();

            if let Ok(o) = output {
                if o.status.success() {
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempt));
                    if output_path.exists() {
                        if let Ok(bytes) = std::fs::read(&output_path) {
                            if image::load_from_memory(&bytes).is_ok() {
                                let _ = std::fs::remove_file(&output_path);
                                return Ok(bytes);
                            }
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        Err("KDE capture failed after 5 attempts".into())
    }

    fn capture_gnome() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let speechd_cache = cache_dir.join("speechd-vision");
        std::fs::create_dir_all(&speechd_cache)?;
        let output_path = speechd_cache.join("capture_gnome.png");
        let output_path_str = output_path.to_string_lossy();

        let status = Command::new("gnome-screenshot")
            .args(["-f", &output_path_str])
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
        let output = Command::new("grim").arg("-").output()?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err("Grim capture failed".into())
        }
    }

    fn capture_x11() -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Try ImageMagick 'import'
        let output = Command::new("import")
            .args(["-window", "root", "png:-"])
            .output();

        if let Ok(o) = output {
            if o.status.success() {
                return Ok(o.stdout);
            }
        }

        // Try scrot
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let speechd_cache = cache_dir.join("speechd-vision");
        std::fs::create_dir_all(&speechd_cache)?;
        let output_path = speechd_cache.join("capture_x11.png");
        let output_path_str = output_path.to_string_lossy();

        let status = Command::new("scrot")
            .args(["--overwrite", &output_path_str])
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

/// The Eye: Moondream 2 Vision-Language Model
struct TheEye {
    model: Option<Model>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    last_used: std::time::Instant,
}

impl TheEye {
    fn new() -> Self {
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);
        println!("Vision service using device: {:?}", device);
        TheEye {
            model: None,
            tokenizer: None,
            device,
            last_used: std::time::Instant::now(),
        }
    }

    fn unload_model(&mut self) {
        if self.model.is_some() {
            println!("Idle timeout: Unloading vision model to free resources...");
            self.model = None;
            self.tokenizer = None;
        }
    }

    fn load_model(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.last_used = std::time::Instant::now();
        if self.model.is_some() {
            return Ok(());
        }

        println!("Loading Moondream 2 (2024-03-06) on {:?}...", self.device);
        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(
            "vikhyatk/moondream2".to_string(),
            RepoType::Model,
            "2024-03-06".to_string(),
        ));

        println!("Fetching model files from HuggingFace...");
        let model_file = repo.get("model.safetensors")?;
        let tokenizer_file = repo.get("tokenizer.json")?;
        println!("Files fetched. Building model (F16)...");

        let config = Config::v2();
        let tokenizer = Tokenizer::from_file(tokenizer_file).map_err(|e| e.to_string())?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_file], DType::F16, &self.device)?
        };

        let model = Model::new(&config, vb)?;
        self.model = Some(model);
        self.tokenizer = Some(tokenizer);

        println!("Moondream 2 ready on {:?}", self.device);
        Ok(())
    }

    fn describe_image(
        &mut self,
        image_bytes: &[u8],
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.model.is_none() {
            self.load_model()?;
        }

        let img_tensor = self.preprocess_image(image_bytes)?;
        let model = self.model.as_mut().unwrap();
        model.text_model.clear_kv_cache();
        let tokenizer = self.tokenizer.as_ref().unwrap();

        let image_embeds = model.vision_encoder.forward(&img_tensor)?;

        let formatted_prompt = format!("\n\nQuestion: {}\n\nAnswer:", prompt);
        let mut tokens = tokenizer
            .encode(formatted_prompt.as_str(), true)
            .map_err(|e| e.to_string())?
            .get_ids()
            .to_vec();

        if tokens.is_empty() {
            return Err("Empty prompt".into());
        }

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

            if let Ok(token_str) = tokenizer.decode(&[next_token], true) {
                if token_str.contains("<END>") || token_str.contains("<|endoftext|>") {
                    break;
                }
                generated_text.push_str(&token_str);
            }

            if next_token == eos_token || tokens.ends_with(&[27, 10619, 29]) {
                break;
            }
        }

        Ok(generated_text
            .replace("<END>", "")
            .replace("<|endoftext|>", "")
            .trim()
            .to_string())
    }

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

        let mean = Tensor::new(&[0.5f32, 0.5, 0.5], &self.device)?.reshape((3, 1, 1))?;
        let std = Tensor::new(&[0.5f32, 0.5, 0.5], &self.device)?.reshape((3, 1, 1))?;

        let img_tensor = img_tensor
            .broadcast_sub(&mean)?
            .broadcast_div(&std)?
            .unsqueeze(0)?
            .to_dtype(DType::F16)?;

        Ok(img_tensor)
    }
}

/// D-Bus service interface for Vision
struct VisionService {
    eye: Arc<Mutex<TheEye>>,
}

#[interface(name = "org.speech.Vision")]
impl VisionService {
    #[zbus(name = "Ping")]
    async fn ping(&self) -> String {
        "pong".to_string()
    }

    #[zbus(name = "GetVersion")]
    async fn get_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[zbus(name = "GetStatus")]
    async fn get_status(&self) -> (bool, String) {
        let eye = self.eye.lock().unwrap();
        let model_loaded = eye.model.is_some();
        let device = format!("{:?}", eye.device);
        (model_loaded, device)
    }

    #[zbus(name = "DescribeScreen")]
    async fn describe_screen(&self, prompt: String) -> String {
        println!("Received DescribeScreen request: {}", prompt);

        let eye = self.eye.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Capture screen
            let image_bytes = match VisionHelper::capture_screen() {
                Ok(bytes) => bytes,
                Err(e) => return format!("Capture Error: {}", e),
            };

            // Analyze with model
            let mut eye_guard = match eye.lock() {
                Ok(guard) => guard,
                Err(_) => return "Error: Vision model lock poisoned".to_string(),
            };

            match eye_guard.describe_image(&image_bytes, &prompt) {
                Ok(desc) => desc,
                Err(e) => format!("Vision Error: {}", e),
            }
        })
        .await;

        match result {
            Ok(s) => s,
            Err(e) => format!("Task Error: {}", e),
        }
    }

    #[zbus(name = "DescribeImage")]
    async fn describe_image(&self, image_base64: String, prompt: String) -> String {
        println!("Received DescribeImage request");

        let eye = self.eye.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Decode base64 image
            let image_bytes = match base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &image_base64,
            ) {
                Ok(bytes) => bytes,
                Err(e) => return format!("Base64 Decode Error: {}", e),
            };

            // Analyze with model
            let mut eye_guard = match eye.lock() {
                Ok(guard) => guard,
                Err(_) => return "Error: Vision model lock poisoned".to_string(),
            };

            match eye_guard.describe_image(&image_bytes, &prompt) {
                Ok(desc) => desc,
                Err(e) => format!("Vision Error: {}", e),
            }
        })
        .await;

        match result {
            Ok(s) => s,
            Err(e) => format!("Task Error: {}", e),
        }
    }

    #[zbus(name = "PreloadModel")]
    async fn preload_model(&self) -> (bool, String) {
        println!("Preloading vision model...");

        let eye = self.eye.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut eye_guard = match eye.lock() {
                Ok(guard) => guard,
                Err(_) => return (false, "Lock poisoned".to_string()),
            };

            match eye_guard.load_model() {
                Ok(_) => (true, "Model loaded successfully".to_string()),
                Err(e) => (false, format!("Load error: {}", e)),
            }
        })
        .await;

        match result {
            Ok(r) => r,
            Err(e) => (false, format!("Task error: {}", e)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("   SpeechD-Vision Service v{}", env!("CARGO_PKG_VERSION"));
    println!("========================================");

    let eye = Arc::new(Mutex::new(TheEye::new()));
    let service = VisionService { eye: eye.clone() }; // Clone for the service, keep original for cleanup task

    let conn = connection::Builder::session()?
        .name("org.speech.Vision")?
        .serve_at("/org/speech/Vision", service)?
        .build()
        .await?;

    println!("Vision service running on D-Bus (org.speech.Vision)");
    println!("Efficiency: Model will auto-unload after 5 minutes of inactivity.");

    // Task for idle-unloading
    let eye_cleanup = eye.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await; // Check every minute
            if let Ok(mut eye_guard) = eye_cleanup.lock() {
                if eye_guard.model.is_some() && eye_guard.last_used.elapsed().as_secs() > 300 {
                    // 5 minutes = 300 seconds
                    eye_guard.unload_model();
                }
            }
        }
    });

    // Keep the service running
    loop {
        std::future::pending::<()>().await;
    }
}
