use speechd_ng::chronicler;
use speechd_ng::config_loader;
use speechd_ng::cortex::Cortex;
use speechd_ng::ear::Ear;
use speechd_ng::engine::AudioEngine;
use speechd_ng::fingerprint::Fingerprint;
use speechd_ng::proactive::ProactiveManager;
use speechd_ng::rate_limiter::RateLimiter;
use speechd_ng::service::SpeechService;
use speechd_ng::ssip;

use clap::Parser;
use std::error::Error;
use std::future::pending;
use std::sync::{Arc, Mutex};
use zbus::connection::Builder;

/// SpeechD-NG: Next-Generation Speech Server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable system load monitoring (CPU/RAM alerts)
    #[arg(long)]
    enable_load_monitor: bool,

    /// Enable desktop notification listener (D-Bus)
    #[arg(long)]
    enable_notifications: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let engine: Arc<dyn speechd_ng::engine::AudioOutput + Send + Sync> =
        Arc::new(AudioEngine::new());
    let ear = Arc::new(Mutex::new(Ear::new()));
    let fingerprint = Fingerprint::new();

    // Initialize Chronicler (Local RAG)
    let chronicler = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let db_path = std::path::PathBuf::from(format!("{}/.local/share/speechd-ng/memory", home));
        std::fs::create_dir_all(&db_path).unwrap_or(());
        Arc::new(chronicler::Chronicler::new(&db_path).expect("Failed to initialize Chronicler"))
    };

    let cortex = Cortex::new(chronicler.clone());

    // Parse CLI arguments
    let args = Args::parse();

    // Initialize Proactive Manager
    let proactive_manager = ProactiveManager::new(cortex.clone(), engine.clone());
    proactive_manager.start_timers().await;

    // Body/Brain Governance: Only enable sensors if explicitly requested
    if args.enable_notifications {
        println!("Governance: Enabling Desktop Notifications (Body Sensor)");
        proactive_manager.start_notification_monitor().await;
    }

    if args.enable_load_monitor {
        println!("Governance: Enabling System Load Monitor (Body Sensor)");
        proactive_manager.start_system_monitor().await;
    }

    // Build the D-Bus connection
    let conn = Builder::session()?
        .name("org.speech.Service")?
        .build()
        .await?;

    // Initialize rate limiter from config
    let config = config_loader::SETTINGS.read().unwrap();
    println!(
        "Loaded Rate Limits - TTS: {}, AI: {}, Audio: {}, Listen: {}",
        config.rate_limit_tts,
        config.rate_limit_ai,
        config.rate_limit_audio,
        config.rate_limit_listen
    );

    let rate_limiter = Arc::new(RateLimiter::new(
        config.rate_limit_tts,
        config.rate_limit_ai,
        config.rate_limit_audio,
        config.rate_limit_listen,
    ));
    drop(config); // Release the read lock

    // Register the service interface with a clone of the connection
    conn.object_server()
        .at(
            "/org/speech/Service",
            SpeechService {
                engine: engine.clone(),
                cortex: cortex.clone(),
                ear: ear.clone(),
                fingerprint: fingerprint.clone(),
                conn: conn.clone(),
                rate_limiter: rate_limiter.clone(),
                model_override: Arc::new(Mutex::new(None)),
            },
        )
        .await?;

    println!("Speech Service running at org.speech.Service");

    // Start SSIP Shim
    let ssip_engine = engine.clone();
    tokio::spawn(async move {
        ssip::start_server(ssip_engine).await;
    });

    // Periodically clean up rate limiter (remove senders inactive for 1 hour)
    let limiter = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(600)); // Every 10 mins
        loop {
            interval.tick().await;
            println!("Background: Cleaning up rate limiter buckets...");
            limiter.cleanup(3600);
        }
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
