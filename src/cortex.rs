use crate::backends::{ollama::OllamaBackend, openai::OpenAIBackend, BrainBackend};
use crate::chronicler::Chronicler;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{channel, Sender};
use tokio::task;

use base64::prelude::*;
use deunicode::deunicode;
use zbus::Connection;

lazy_static! {
    static ref INJECTION_REGEX: Regex = Regex::new(
        r"(?i)ignore\s+previous|disregard|system:|assistant:|user:|\bshell\b|exec\b|sudo\b|root\b"
    )
    .unwrap();
}

fn sanitize_input(input: &str) -> String {
    let mut sanitized = input.replace("```", "");
    let normalized = deunicode(&sanitized);
    if INJECTION_REGEX.is_match(&normalized) {
        sanitized = INJECTION_REGEX
            .replace_all(&sanitized, "[FILTERED]")
            .to_string();
    }
    sanitized
}

fn get_memory_size() -> usize {
    crate::config_loader::SETTINGS
        .read()
        .map(|s| s.memory_size)
        .unwrap_or(50)
}

#[derive(Clone)]
pub struct Cortex {
    tx: Sender<CortexMessage>,
}

enum CortexMessage {
    Observe(String),
    Query {
        prompt: String,
        _asr_heard: Option<String>,
        _images: Option<Vec<String>>,
        response_tx: Sender<String>,
    },
    QueryStream {
        prompt: String,
        _asr_heard: Option<String>,
        images: Option<Vec<String>>,
        token_tx: Sender<String>,
    },
    VisualQueryLocal {
        prompt: String,
        response_tx: Sender<String>,
    },
}

struct Memory {
    history: VecDeque<String>,
    max_size: usize,
}

impl Memory {
    fn new() -> Self {
        let max_size = get_memory_size();
        Self {
            history: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn add(&mut self, text: String) {
        if self.history.len() >= self.max_size {
            self.history.pop_front();
        }
        self.history.push_back(text);
    }

    fn get_context(&self) -> String {
        self.history
            .iter()
            .cloned()
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl Cortex {
    pub fn new_dummy() -> Self {
        let (tx, mut rx) = channel::<CortexMessage>(100);
        tokio::spawn(async move { while rx.recv().await.is_some() {} });
        Self { tx }
    }

    pub fn new_testing() -> Self {
        let (tx, mut rx) = channel::<CortexMessage>(100);
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let CortexMessage::QueryStream { token_tx, .. } = msg {
                    let _ = token_tx.send("Testing response.".to_string()).await;
                }
            }
        });
        Self { tx }
    }

    pub fn new(chronicler: Arc<Chronicler>) -> Self {
        let (tx, mut rx) = channel::<CortexMessage>(100);
        let memory = Arc::new(Mutex::new(Memory::new()));
        let chron = chronicler.clone();
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        let backend: Arc<dyn BrainBackend + Send + Sync> = {
            let s = crate::config_loader::SETTINGS.read().unwrap();
            let bitnet = Arc::new(OpenAIBackend::new(
                s.bitnet_url.clone(),
                s.bitnet_model.clone(),
                client.clone(),
            ));
            let ollama = Arc::new(OllamaBackend::new(
                s.ollama_url.clone(),
                s.ollama_model.clone(),
                client.clone(),
            ));

            match s.ai_backend.as_str() {
                "bitnet" => bitnet,
                "ollama" => ollama,
                "auto" => Arc::new(crate::backends::fallback::FallbackBackend::new(
                    bitnet, ollama,
                )),
                _ => bitnet, // Default to BitNet as primary interface
            }
        };

        task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    CortexMessage::Observe(text) => {
                        let text = sanitize_input(&text);
                        println!("Cortex observing: {}", text);
                        if let Ok(mut mem) = memory.lock() {
                            mem.add(text.clone());
                        }
                        if let Err(e) = chron.add_memory(&text) {
                            eprintln!("Cortex: Failed to add memory: {}", e);
                        }
                    }
                    CortexMessage::VisualQueryLocal {
                        prompt,
                        response_tx,
                    } => {
                        let result = async {
                            let (enabled, prompt) = {
                                let s = crate::config_loader::SETTINGS.read().unwrap();
                                (s.enable_vision, sanitize_input(&prompt))
                            };

                            if !enabled {
                                return Ok(
                                    "Vision features are disabled in configuration.".to_string()
                                );
                            }

                            let conn = Connection::session().await?;
                            let proxy = conn
                                .call_method(
                                    Some("org.speech.Vision"),
                                    "/org/speech/Vision",
                                    Some("org.speech.Vision"),
                                    "DescribeScreen",
                                    &(prompt,),
                                )
                                .await?;
                            let response: String = proxy.body().deserialize()?;
                            Ok::<String, zbus::Error>(response)
                        }
                        .await;

                        let response = match result {
                            Ok(desc) => desc,
                            Err(e) => {
                                let err_str = e.to_string();
                                if err_str.contains("ServiceUnknown") {
                                    "Vision service not running. Install and start speechd-vision for screen description features.".to_string()
                                } else {
                                    format!("Vision Error: {}", e)
                                }
                            }
                        };
                        let _ = response_tx.send(response).await;
                    }
                    CortexMessage::Query {
                        prompt,
                        _asr_heard: _,
                        _images: _,
                        response_tx,
                    } => {
                        let prompt = sanitize_input(&prompt);
                        let ai_enabled = crate::config_loader::SETTINGS
                            .read()
                            .map(|s| s.enable_ai)
                            .unwrap_or(true);

                        if !ai_enabled {
                            let _ = response_tx.send("AI is disabled.".to_string()).await;
                            continue;
                        }

                        let mut context = if let Ok(mem) = memory.lock() {
                            mem.get_context()
                        } else {
                            String::new()
                        };

                        if let Ok(past_memories) = chron.search(&prompt, 3) {
                            if !past_memories.is_empty() {
                                context.push_str("\n--- LONG-TERM MEMORIES ---\n");
                                for mem in past_memories {
                                    context.push_str(&format!("- {}\n", mem));
                                }
                            }
                        }

                        let system_prompt = crate::config_loader::SETTINGS
                            .read()
                            .unwrap()
                            .system_prompt
                            .clone();

                        match backend.prompt(&system_prompt, &context, &prompt).await {
                            Ok(response) => {
                                let _ = response_tx.send(response).await;
                            }
                            Err(e) => {
                                let _ = response_tx.send(format!("Backend Error: {}", e)).await;
                            }
                        }
                    }
                    CortexMessage::QueryStream {
                        prompt,
                        _asr_heard: _,
                        images,
                        token_tx,
                    } => {
                        let mut context = if let Ok(mem) = memory.lock() {
                            mem.get_context()
                        } else {
                            String::new()
                        };

                        if let Ok(past_memories) = chron.search(&prompt, 3) {
                            if !past_memories.is_empty() {
                                context.push_str("\n--- LONG-TERM MEMORIES ---\n");
                                for mem in past_memories {
                                    context.push_str(&format!("- {}\n", mem));
                                }
                            }
                        }

                        let system_prompt = crate::config_loader::SETTINGS
                            .read()
                            .unwrap()
                            .system_prompt
                            .clone();

                        let mut stream_rx = backend
                            .stream(&system_prompt, &context, &prompt, images)
                            .await;
                        while let Some(token) = stream_rx.recv().await {
                            let _ = token_tx.send(token).await;
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn observe(&self, text: String) {
        let _ = self.tx.send(CortexMessage::Observe(text)).await;
    }

    pub async fn query(&self, prompt: String) -> String {
        let (resp_tx, mut resp_rx) = channel::<String>(1);
        let _ = self
            .tx
            .send(CortexMessage::Query {
                prompt,
                _asr_heard: None,
                _images: None,
                response_tx: resp_tx,
            })
            .await;
        resp_rx
            .recv()
            .await
            .unwrap_or_else(|| "Internal Error".into())
    }

    pub async fn query_with_vision(&self, prompt: String, image_bytes: Option<Vec<u8>>) -> String {
        let (resp_tx, mut resp_rx) = channel::<String>(1);
        let images = image_bytes.map(|bytes| vec![BASE64_STANDARD.encode(&bytes)]);

        let _ = self
            .tx
            .send(CortexMessage::Query {
                prompt,
                _asr_heard: None,
                _images: images,
                response_tx: resp_tx,
            })
            .await;
        resp_rx
            .recv()
            .await
            .unwrap_or_else(|| "Internal Error".into())
    }

    pub async fn query_stream(
        &self,
        prompt: String,
        images: Option<Vec<String>>,
    ) -> tokio::sync::mpsc::Receiver<String> {
        let (token_tx, token_rx) = channel::<String>(100);
        let _ = self
            .tx
            .send(CortexMessage::QueryStream {
                prompt,
                _asr_heard: None,
                images,
                token_tx,
            })
            .await;
        token_rx
    }

    pub async fn query_local_vision(&self, prompt: String) -> String {
        let (resp_tx, mut resp_rx) = channel::<String>(1);
        let _ = self
            .tx
            .send(CortexMessage::VisualQueryLocal {
                prompt,
                response_tx: resp_tx,
            })
            .await;
        resp_rx
            .recv()
            .await
            .unwrap_or_else(|| "Internal Error".into())
    }
}
