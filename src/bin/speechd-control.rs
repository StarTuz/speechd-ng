//! speechd-control - CLI control utility for SpeechD-NG daemon
//!
//! A command-line interface for interacting with the speechd-ng daemon via D-Bus.

use clap::{Parser, Subcommand};
use zbus::blocking::Connection;

/// CLI control utility for SpeechD-NG daemon
#[derive(Parser)]
#[command(name = "speechd-control")]
#[command(author = "StarTuz")]
#[command(version)]
#[command(about = "Control utility for the SpeechD-NG speech daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Speak text using TTS
    Speak {
        /// Text to speak
        text: String,
        /// Voice to use (optional)
        #[arg(short, long)]
        voice: Option<String>,
        /// Audio channel: left, right, center, stereo (optional)
        #[arg(short, long)]
        channel: Option<String>,
    },

    /// Listen and transcribe speech (VAD mode)
    Listen,

    /// Query the AI about speech context
    Think {
        /// Question to ask
        query: String,
    },

    /// Show daemon status
    Status,

    /// Check service health
    Ping,

    /// Get daemon version
    Version,

    /// List available voices
    Voices,

    /// Play audio from URL
    Play {
        /// URL to audio file
        url: String,
        /// Audio channel (optional)
        #[arg(short, long)]
        channel: Option<String>,
    },

    /// Stop current audio playback
    Stop,

    /// Set or get volume
    Volume {
        /// Volume level (0.0-1.0), omit to get current volume
        level: Option<f64>,
    },

    /// Train a word for voice recognition
    Train {
        /// Word to train
        word: String,
        /// Recording duration in seconds
        #[arg(short, long, default_value = "3")]
        duration: u32,
    },

    /// List learned voice patterns
    Patterns,

    /// Add a voice correction
    Correct {
        /// What ASR hears incorrectly
        heard: String,
        /// What you actually meant
        meant: String,
    },

    /// Undo last voice correction
    Rollback,

    /// AI brain management
    Brain {
        #[command(subcommand)]
        action: Option<BrainAction>,
    },

    /// List audio output devices
    Sinks,
}

#[derive(Subcommand)]
enum BrainAction {
    /// Start Ollama service
    Start,
    /// Stop Ollama service
    Stop,
    /// Pull a model
    Pull {
        /// Model name (e.g., llama3, mistral)
        model: String,
    },
    /// Switch to a different model
    Use {
        /// Model name to use (e.g., llama3, mistral)
        model: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let conn = Connection::session()?;

    let dest = "org.speech.Service";
    let path = "/org/speech/Service";
    let iface = "org.speech.Service";

    match cli.command {
        Commands::Speak {
            text,
            voice,
            channel,
        } => {
            let text_display = text.clone();
            match (voice, channel) {
                (Some(v), Some(c)) => {
                    let _: bool = conn
                        .call_method(Some(dest), path, Some(iface), "SpeakChannel", &(text, v, c))?
                        .body()
                        .deserialize()?;
                }
                (Some(v), None) => {
                    let _: () = conn
                        .call_method(Some(dest), path, Some(iface), "SpeakVoice", &(text, v))?
                        .body()
                        .deserialize()?;
                }
                (None, Some(c)) => {
                    let _: bool = conn
                        .call_method(
                            Some(dest),
                            path,
                            Some(iface),
                            "SpeakChannel",
                            &(text, String::new(), c),
                        )?
                        .body()
                        .deserialize()?;
                }
                (None, None) => {
                    let _: () = conn
                        .call_method(Some(dest), path, Some(iface), "Speak", &text)?
                        .body()
                        .deserialize()?;
                }
            }
            println!("Speaking: {}", text_display);
        }

        Commands::Listen => {
            println!("Listening (VAD mode)...");
            let result: String = conn
                .call_method(Some(dest), path, Some(iface), "ListenVad", &())?
                .body()
                .deserialize()?;
            println!("Heard: {}", result);
        }

        Commands::Think { query } => {
            println!("Thinking...");
            let result: String = conn
                .call_method(Some(dest), path, Some(iface), "Think", &query)?
                .body()
                .deserialize()?;
            println!("{}", result);
        }

        Commands::Status => {
            let (ai, thresh, stt, patterns): (bool, f32, String, u32) = conn
                .call_method(Some(dest), path, Some(iface), "GetStatus", &())?
                .body()
                .deserialize()?;

            let version: String = conn
                .call_method(Some(dest), path, Some(iface), "GetVersion", &())?
                .body()
                .deserialize()?;

            println!("SpeechD-NG Status");
            println!("─────────────────");
            println!("Version:      {}", version);
            println!("AI Enabled:   {}", if ai { "Yes" } else { "No" });
            println!("STT Backend:  {}", stt);
            println!("Patterns:     {}", patterns);
            println!("Threshold:    {:.0}%", thresh * 100.0);
        }

        Commands::Ping => {
            let result: String = conn
                .call_method(Some(dest), path, Some(iface), "Ping", &())?
                .body()
                .deserialize()?;
            println!("{}", result);
        }

        Commands::Version => {
            let result: String = conn
                .call_method(Some(dest), path, Some(iface), "GetVersion", &())?
                .body()
                .deserialize()?;
            println!("speechd-ng {}", result);
        }

        Commands::Voices => {
            let voices: Vec<(String, String)> = conn
                .call_method(Some(dest), path, Some(iface), "ListVoices", &())?
                .body()
                .deserialize()?;

            if voices.is_empty() {
                println!("No voices installed");
            } else {
                println!("Installed Voices");
                println!("────────────────");
                for (id, name) in voices {
                    println!("  {} ({})", name, id);
                }
            }
        }

        Commands::Play { url, channel } => {
            let result: String = match channel {
                Some(c) => conn
                    .call_method(
                        Some(dest),
                        path,
                        Some(iface),
                        "PlayAudioChannel",
                        &(url.clone(), c),
                    )?
                    .body()
                    .deserialize()?,
                None => conn
                    .call_method(Some(dest), path, Some(iface), "PlayAudio", &url)?
                    .body()
                    .deserialize()?,
            };

            if result.is_empty() {
                println!("Playing: {}", url);
            } else {
                eprintln!("Error: {}", result);
            }
        }

        Commands::Stop => {
            let stopped: bool = conn
                .call_method(Some(dest), path, Some(iface), "StopAudio", &())?
                .body()
                .deserialize()?;

            if stopped {
                println!("Playback stopped");
            } else {
                println!("Nothing was playing");
            }
        }

        Commands::Volume { level } => match level {
            Some(vol) => {
                let _: bool = conn
                    .call_method(Some(dest), path, Some(iface), "SetVolume", &vol)?
                    .body()
                    .deserialize()?;
                println!("Volume set to {:.0}%", vol * 100.0);
            }
            None => {
                let vol: f64 = conn
                    .call_method(Some(dest), path, Some(iface), "GetVolume", &())?
                    .body()
                    .deserialize()?;
                println!("Volume: {:.0}%", vol * 100.0);
            }
        },

        Commands::Train { word, duration } => {
            println!("Say '{}' in {} seconds...", word, duration);
            let (heard, success): (String, bool) = conn
                .call_method(
                    Some(dest),
                    path,
                    Some(iface),
                    "TrainWord",
                    &(word.clone(), duration),
                )?
                .body()
                .deserialize()?;

            if success {
                println!("✓ Learned: '{}' → '{}'", heard, word);
            } else {
                println!("✗ Training failed. Heard: '{}'", heard);
            }
        }

        Commands::Patterns => {
            let patterns: Vec<(String, String, String)> = conn
                .call_method(Some(dest), path, Some(iface), "ListPatterns", &())?
                .body()
                .deserialize()?;

            if patterns.is_empty() {
                println!("No patterns learned yet");
            } else {
                println!("Learned Patterns");
                println!("────────────────");
                for (heard, meant, conf) in patterns {
                    println!("  '{}' → '{}' [{}]", heard, meant, conf);
                }
            }
        }

        Commands::Correct { heard, meant } => {
            let success: bool = conn
                .call_method(
                    Some(dest),
                    path,
                    Some(iface),
                    "AddCorrection",
                    &(heard.clone(), meant.clone()),
                )?
                .body()
                .deserialize()?;

            if success {
                println!("✓ Added correction: '{}' → '{}'", heard, meant);
            } else {
                println!("✗ Failed to add correction");
            }
        }

        Commands::Rollback => {
            let success: bool = conn
                .call_method(Some(dest), path, Some(iface), "RollbackLastCorrection", &())?
                .body()
                .deserialize()?;

            if success {
                println!("✓ Rolled back last correction");
            } else {
                println!("✗ No correction to rollback");
            }
        }

        Commands::Brain { action } => {
            match action {
                None => {
                    // Show brain status
                    let (running, model, models): (bool, String, Vec<String>) = conn
                        .call_method(Some(dest), path, Some(iface), "GetBrainStatus", &())?
                        .body()
                        .deserialize()?;

                    println!("AI Brain Status");
                    println!("───────────────");
                    println!("Status:    {}", if running { "Online" } else { "Offline" });
                    println!("Model:     {}", model);
                    if !models.is_empty() {
                        println!("Available: {}", models.join(", "));
                    }
                }
                Some(BrainAction::Start) => {
                    let success: bool = conn
                        .call_method(Some(dest), path, Some(iface), "ManageBrain", &("start", ""))?
                        .body()
                        .deserialize()?;
                    println!(
                        "{}",
                        if success {
                            "Starting Ollama..."
                        } else {
                            "Failed to start"
                        }
                    );
                }
                Some(BrainAction::Stop) => {
                    let success: bool = conn
                        .call_method(Some(dest), path, Some(iface), "ManageBrain", &("stop", ""))?
                        .body()
                        .deserialize()?;
                    println!(
                        "{}",
                        if success {
                            "Stopping Ollama..."
                        } else {
                            "Failed to stop"
                        }
                    );
                }
                Some(BrainAction::Pull { model }) => {
                    println!("Pulling model '{}'...", model);
                    let success: bool = conn
                        .call_method(
                            Some(dest),
                            path,
                            Some(iface),
                            "ManageBrain",
                            &("pull", model.as_str()),
                        )?
                        .body()
                        .deserialize()?;
                    println!(
                        "{}",
                        if success {
                            "Pull initiated"
                        } else {
                            "Failed to pull"
                        }
                    );
                }
                Some(BrainAction::Use { model }) => {
                    let success: bool = conn
                        .call_method(Some(dest), path, Some(iface), "SetBrainModel", &model)?
                        .body()
                        .deserialize()?;
                    if success {
                        println!("✓ Switched to model: {}", model);
                    } else {
                        println!("✗ Failed to switch model. Is '{}' installed?", model);
                    }
                }
            }
        }

        Commands::Sinks => {
            let sinks: Vec<(u32, String, String, bool)> = conn
                .call_method(Some(dest), path, Some(iface), "ListSinks", &())?
                .body()
                .deserialize()?;

            if sinks.is_empty() {
                println!("No audio sinks found");
            } else {
                println!("Audio Output Devices");
                println!("────────────────────");
                for (id, name, _desc, is_default) in sinks {
                    let marker = if is_default { "*" } else { " " };
                    println!("{} [{}] {}", marker, id, name);
                }
                println!("\n* = default");
            }
        }
    }

    Ok(())
}
