mod engine;
mod cortex;
mod config_loader;
mod security;
use engine::AudioEngine;
use cortex::Cortex;
use security::SecurityAgent;
use std::error::Error;
use std::future::pending;
use std::sync::{Arc, Mutex};
use zbus::{interface, connection::Builder, message::Header};

struct SpeechService {
    engine: Arc<Mutex<AudioEngine>>,
    cortex: Cortex,
}

#[interface(name = "org.speech.Service")]
impl SpeechService {
    async fn speak(&self, #[zbus(header)] header: Header<'_>, text: String) {
        // Enforce basic permissions (Optional, maybe speaking is always allowed?)
        // SecurityAgent::check_permission(&header, "org.speech.service.speak").await.unwrap();
        
        println!("Received speak request: {}", text);
        
        // Parallel Dispatch: Body speaks, Brain remembers.
        if let Ok(engine) = self.engine.lock() {
            engine.speak(&text);
        }
        self.cortex.observe(text).await;
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let engine = Arc::new(Mutex::new(AudioEngine::new()));
    let cortex = Cortex::new();

    let _conn = Builder::session()?
        .name("org.speech.Service")?
        .serve_at("/org/speech/Service", SpeechService { engine, cortex })?
        .build()
        .await?;

    println!("Speech Service running at org.speech.Service");

    // Keep the service running
    pending::<()>().await;

    Ok(())
}
