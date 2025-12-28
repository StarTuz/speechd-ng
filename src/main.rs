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
        
        // Check if audio is enabled
        let audio_enabled = config_loader::SETTINGS.read()
            .map(|s| s.enable_audio)
            .unwrap_or(true);
        
        // Parallel Dispatch: Body speaks (if enabled), Brain remembers.
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
        // We return a tuple of (ID, Name) for simplicity over D-Bus
        let voices = if let Ok(engine) = self.engine.lock() {
            // Since list_voices is async, and we are inside a sync mutex lock, we can't await it easily HERE 
            // if list_voices is on the engine struct. 
            // BUT engine.list_voices() returns a Future.
            // We need to clone the logic out or just not hold the lock while awaiting.
            // Ideally: Clone the engine (it's Arc<Mutex> wrapper? No, AudioEngine is Clone via logic?)
            // AudioEngine is Clone (it just has a sender).
             Some(engine.clone())
        } else {
            None
        };

        if let Some(engine) = voices {
             let list = engine.list_voices().await;
             list.into_iter().map(|v| (v.id, v.name)).collect()
        } else {
             Vec::new()
        }
    }

    async fn think(&self, #[zbus(header)] header: Header<'_>, query: String) -> String {
        // STRICT SECURITY: Thinking accesses history, so we MUST check permissions.
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.think").await {
            eprintln!("Access Denied: {}", e);
            return "Access Denied".to_string();
        }

        println!("Received thought query: {}", query);
        self.cortex.query(query).await
    }

    async fn listen(&self, #[zbus(header)] header: Header<'_>) -> String {
        // STRICT SECURITY: Listening activates the microphone. Must be authenticated.
        if let Err(e) = SecurityAgent::check_permission(&header, "org.speech.service.listen").await {
            eprintln!("Access Denied: {}", e);
            return "Access Denied".to_string();
        }

        println!("Received listen request");
        
        // Offload blocking audio capture to a dedicated thread to avoid starving the async runtime
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
        .serve_at("/org/speech/Service", SpeechService { engine: engine.clone(), cortex, ear })?
        .build()
        .await?;

    println!("Speech Service running at org.speech.Service");

    // Start SSIP Shim (Legacy Compatibility)
    let ssip_engine = engine.clone();
    tokio::spawn(async move {
        ssip::start_server(ssip_engine).await;
    });

    // Keep the service running
    pending::<()>().await;

    Ok(())
}
