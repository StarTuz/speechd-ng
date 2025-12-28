mod engine;
mod cortex;
mod config_loader;
mod security;
mod backends;
mod ear;
mod ssip;
use engine::AudioEngine;
use cortex::Cortex;
use ear::Ear;
use security::SecurityAgent;
use std::error::Error;
use std::future::pending;
use std::sync::{Arc, Mutex};
use zbus::{interface, connection::Builder, message::Header};

struct SpeechService {
    engine: Arc<Mutex<AudioEngine>>,
    cortex: Cortex,
    ear: Arc<Mutex<Ear>>,
}

#[interface(name = "org.speech.Service")]
impl SpeechService {
    async fn speak(&self, #[zbus(header)] _header: Header<'_>, text: String) {
        println!("Received speak request: {}", text);
        
        let audio_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);
        
        if audio_enabled {
            if let Ok(engine) = self.engine.lock() {
                engine.speak(&text, None);
            }
        }
        self.cortex.observe(text).await;
    }

    async fn speak_voice(&self, #[zbus(header)] _header: Header<'_>, text: String, voice: String) {
         println!("Received speak request (voice: {}): {}", voice, text);
         
         let audio_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);

         if audio_enabled {
             if let Ok(engine) = self.engine.lock() {
                 engine.speak(&text, Some(voice));
             }
         }
         self.cortex.observe(text).await;
    }

    async fn list_voices(&self) -> Vec<(String, String)> {
        let engine = if let Ok(engine) = self.engine.lock() {
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = engine {
             let list = engine.list_voices().await;
             list.into_iter().map(|v| (v.id, v.name)).collect()
        } else {
             Vec::new()
        }
    }

    async fn list_downloadable_voices(&self) -> Vec<(String, String)> {
        let engine = if let Ok(engine) = self.engine.lock() {
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = engine {
             let list = engine.list_downloadable_voices().await;
             list.into_iter().map(|v| (v.id, format!("{} [{}]", v.name, v.language))).collect()
        } else {
             Vec::new()
        }
    }

    async fn download_voice(&self, #[zbus(header)] header: Header<'_>, voice_id: String) -> String {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.manage").await {
            return format!("Access Denied: {}", e);
        }

        let engine = if let Ok(engine) = self.engine.lock() {
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = engine {
            match engine.download_voice(voice_id).await {
                Ok(_) => "Success".to_string(),
                Err(e) => format!("Error: {}", e),
            }
        } else {
             "Error: Engine locked".to_string()
        }
    }

    async fn think(&self, #[zbus(header)] header: Header<'_>, query: String) -> String {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.think").await {
            eprintln!("Access Denied: {}", e);
            return "Access Denied".to_string();
        }

        println!("Received thought query: {}", query);
        self.cortex.query(query).await
    }

    async fn listen(&self, #[zbus(header)] header: Header<'_>) -> String {
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.listen").await {
            eprintln!("Access Denied: {}", e);
            return "Access Denied".to_string();
        }

        println!("Received listen request");
        
        let ear = self.ear.clone();
        let result = tokio::task::spawn_blocking(move || {
            if let Ok(ear_guard) = ear.lock() {
                ear_guard.listen()
            } else {
                "Error: Ear locked".to_string()
            }
        }).await;

        match result {
            Ok(s) => s,
            Err(e) => format!("Error joining audio task: {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let engine = Arc::new(Mutex::new(AudioEngine::new()));
    let cortex = Cortex::new();
    let ear = Arc::new(Mutex::new(Ear::new()));

    let _conn = Builder::session()?
        .name("org.speech.Service")?
        .serve_at("/org/speech/Service", SpeechService { 
            engine: engine.clone(), 
            cortex: cortex.clone(), 
            ear: ear.clone() 
        })?
        .build()
        .await?;

    println!("Speech Service running at org.speech.Service");

    // Start SSIP Shim
    let ssip_engine = engine.clone();
    tokio::spawn(async move {
        ssip::start_server(ssip_engine).await;
    });

    // Start Autonomous Mode (Wake Word + Command Processing)
    let config = config_loader::SETTINGS.read().unwrap();
    if config.enable_wake_word {
         let ear_handler = ear.clone();
         let engine_handler = engine.clone();
         let cortex_handler = cortex.clone();
         tokio::task::spawn_blocking(move || {
             if let Ok(ear_guard) = ear_handler.lock() {
                 ear_guard.start_autonomous_mode(engine_handler, cortex_handler);
             }
         });
    }

    pending::<()>().await;

    Ok(())
}
