use crate::chronicler::Chronicler;
use crate::fingerprint::Fingerprint;
use crate::vision::{TheEye, VisionHelper};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{channel, Sender};
use tokio::task;

use base64::prelude::*;
use deunicode::deunicode;

lazy_static! {
    static ref INJECTION_REGEX: Regex = Regex::new(
        r"(?i)ignore\s+previous|disregard|system:|assistant:|user:|\bshell\b|exec\b|sudo\b|root\b"
    )
    .unwrap();
}

fn sanitize_input(input: &str) -> String {
    // 1. Remove obvious code blocks
    let mut sanitized = input.replace("```", "");

    // 2. Normalize homoglyphs using deunicode
    let normalized = deunicode(&sanitized);

    // 3. Filter against injection patterns on normalized text
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
    Observe(String), // Passive: Just listen and remember
    Query {
        prompt: String,
        _asr_heard: Option<String>,
        _images: Option<Vec<String>>,
        response_tx: Sender<String>,
    }, // Active: Ask a question about context
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
        tokio::spawn(async move { while let Some(_) = rx.recv().await {} });
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

        let _fingerprint = Fingerprint::new();

        task::spawn(async move {
            let mut eye = TheEye::new();
            while let Some(msg) = rx.recv().await {
                match msg {
                    CortexMessage::Observe(text) => {
                        let text = sanitize_input(&text);
                        println!("Cortex observing: {}", text);
                        if let Ok(mut mem) = memory.lock() {
                            mem.add(text.clone());
                        }
                        // Add to long-term memory (RAG)
                        if let Err(e) = chron.add_memory(&text) {
                            eprintln!("Cortex: Failed to add memory: {}", e);
                        }
                    }
                    CortexMessage::VisualQueryLocal {
                        prompt,
                        response_tx,
                    } => {
                        let prompt = sanitize_input(&prompt);
                        let image_result = VisionHelper::capture_screen();
                        match image_result {
                            Ok(bytes) => match eye.describe_image(&bytes, &prompt) {
                                Ok(desc) => {
                                    let _ = response_tx.send(desc).await;
                                }
                                Err(e) => {
                                    let _ = response_tx.send(format!("Vision Error: {}", e)).await;
                                }
                            },
                            Err(e) => {
                                let _ = response_tx
                                    .send(format!("Failed to capture screen: {}", e))
                                    .await;
                            }
                        }
                    }
                    CortexMessage::Query {
                        prompt,
                        _asr_heard: _,
                        _images: _,
                        response_tx,
                    } => {
                        let prompt = sanitize_input(&prompt);
                        // Check if AI is enabled
                        let ai_enabled = crate::config_loader::SETTINGS
                            .read()
                            .map(|s| s.enable_ai)
                            .unwrap_or(true);

                        if !ai_enabled {
                            let _ = response_tx.send("AI is disabled.".to_string()).await;
                            continue;
                        }

                        println!("Cortex thinking on: {}", prompt);

                        let mut context = if let Ok(mem) = memory.lock() {
                            mem.get_context()
                        } else {
                            String::new()
                        };

                        // Retrieve from long-term memory (RAG)
                        if let Ok(past_memories) = chron.search(&prompt, 3) {
                            if !past_memories.is_empty() {
                                context.push_str("\n--- LONG-TERM MEMORIES ---\n");
                                for mem in past_memories {
                                    context.push_str(&format!("- {}\n", mem));
                                }
                            }
                        }

                        let payload = json!({
                            "model": "llama3.2:3b",
                            "prompt": format!("Context:\n{}\n\nUser: {}", context, prompt),
                            "stream": false
                        });

                        match client
                            .post("http://localhost:11434/api/generate")
                            .json(&payload)
                            .send()
                            .await
                        {
                            Ok(res) => {
                                if let Ok(json) = res.json::<serde_json::Value>().await {
                                    let response = json["response"]
                                        .as_str()
                                        .unwrap_or("No response from AI.")
                                        .to_string();
                                    let _ = response_tx.send(response).await;
                                } else {
                                    let _ = response_tx
                                        .send("Failed to parse AI response.".into())
                                        .await;
                                }
                            }
                            Err(e) => {
                                let _ = response_tx.send(format!("AI Error: {}", e)).await;
                            }
                        }
                    }
                    CortexMessage::QueryStream {
                        prompt,
                        _asr_heard: _,
                        images,
                        token_tx,
                    } => {
                        // Implementation for streaming...
                        let mut context = if let Ok(mem) = memory.lock() {
                            mem.get_context()
                        } else {
                            String::new()
                        };

                        // Retrieve from long-term memory (RAG)
                        if let Ok(past_memories) = chron.search(&prompt, 3) {
                            if !past_memories.is_empty() {
                                context.push_str("\n--- LONG-TERM MEMORIES ---\n");
                                for mem in past_memories {
                                    context.push_str(&format!("- {}\n", mem));
                                }
                            }
                        }

                        let payload = json!({
                            "model": "llama3.2:3b",
                            "prompt": format!("Context:\n{}\n\nUser: {}", context, prompt),
                            "stream": true,
                            "images": images
                        });

                        let res = client
                            .post("http://localhost:11434/api/generate")
                            .json(&payload)
                            .send()
                            .await;

                        match res {
                            Ok(response) => {
                                let mut stream = response.bytes_stream();
                                use futures_util::StreamExt;
                                while let Some(item) = stream.next().await {
                                    if let Ok(chunk) = item {
                                        if let Ok(json) =
                                            serde_json::from_slice::<serde_json::Value>(&chunk)
                                        {
                                            if let Some(token) = json["response"].as_str() {
                                                let _ = token_tx.send(token.to_string()).await;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = token_tx.send(format!("Stream Error: {}", e)).await;
                            }
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
