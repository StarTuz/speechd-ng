use tokio::sync::mpsc::{channel, Sender};
use tokio::task;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use reqwest::Client;
use serde_json::json;

fn get_memory_size() -> usize {
    crate::config_loader::SETTINGS.read()
        .map(|s| s.memory_size)
        .unwrap_or(50)
}

#[derive(Clone)]
pub struct Cortex {
    tx: Sender<CortexMessage>,
}

enum CortexMessage {
    Observe(String),     // Passive: Just listen and remember
    Query(String, Sender<String>), // Active: Ask a question about context
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
        self.history.iter().cloned().collect::<Vec<String>>().join("\n")
    }
}

impl Cortex {
    pub fn new() -> Self {
        let (tx, mut rx) = channel::<CortexMessage>(100);
        let memory = Arc::new(Mutex::new(Memory::new()));
        let client = Client::new();

        task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    CortexMessage::Observe(text) => {
                        println!("Cortex observing: {}", text);
                        if let Ok(mut mem) = memory.lock() {
                            mem.add(text);
                        }
                    }
                    CortexMessage::Query(prompt, response_tx) => {
                        println!("Cortex thinking on: {}", prompt);
                        
                        let context = if let Ok(mem) = memory.lock() {
                            mem.get_context()
                        } else {
                            String::new()
                        };

                        // Construct the full prompt with context
                        // This assumes Ollama contains a 'llama3' or 'mistral' model, we default to generic
                        let full_prompt = format!(
                            "Context of recent speech:\n{}\n\nUser Question: {}", 
                            context, prompt
                        );

                        // Call Ollama
                        let (url, model) = {
                            let settings = crate::config_loader::SETTINGS.read().unwrap();
                            (settings.ollama_url.clone(), settings.ollama_model.clone())
                        };

                        let res = client.post(&format!("{}/api/generate", url))
                            .json(&json!({
                                "model": model, 
                                "prompt": full_prompt,
                                "stream": false
                            }))
                            .send()
                            .await;

                        let answer = match res {
                            Ok(resp) => {
                                if let Ok(json) = resp.json::<serde_json::Value>().await {
                                    json["response"].as_str().unwrap_or("I'm confused.").to_string()
                                } else {
                                    "Failed to parse AI response.".to_string()
                                }
                            },
                            Err(_) => "Could not contact Ollama (Brain offline).".to_string(),
                        };

                        let _ = response_tx.send(answer).await;
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
        let _ = self.tx.send(CortexMessage::Query(prompt, resp_tx)).await;
        resp_rx.recv().await.unwrap_or_else(|| "Internal Error".into())
    }
}
