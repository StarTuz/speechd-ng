use crate::backends::BrainBackend;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc::{channel, Receiver};

pub struct OllamaBackend {
    pub url: String,
    pub model: String,
    pub client: Client,
}

impl OllamaBackend {
    pub fn new(url: String, model: String, client: Client) -> Self {
        Self { url, model, client }
    }
}

#[async_trait]
impl BrainBackend for OllamaBackend {
    async fn prompt(&self, system: &str, context: &str, user: &str) -> Result<String, String> {
        let payload = json!({
            "model": self.model,
            "system": system,
            "prompt": format!("Context:\n{}\n\nUser: {}", context, user),
            "stream": false
        });

        match self
            .client
            .post(format!("{}/api/generate", self.url))
            .json(&payload)
            .send()
            .await
        {
            Ok(res) => {
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    Ok(json["response"]
                        .as_str()
                        .unwrap_or("No response from AI.")
                        .to_string())
                } else {
                    Err("Failed to parse AI response.".into())
                }
            }
            Err(e) => Err(format!("AI Error: {}", e)),
        }
    }

    async fn stream(
        &self,
        system: &str,
        context: &str,
        user: &str,
        images: Option<Vec<String>>,
    ) -> Receiver<String> {
        let (token_tx, token_rx) = channel::<String>(100);
        let url = format!("{}/api/generate", self.url);
        let client = self.client.clone();
        let payload = json!({
            "model": self.model,
            "system": system,
            "prompt": format!("Context:\n{}\n\nUser: {}", context, user),
            "stream": true,
            "images": images
        });

        tokio::spawn(async move {
            let res = client.post(url).json(&payload).send().await;

            match res {
                Ok(response) => {
                    let mut stream = response.bytes_stream();
                    use futures_util::StreamExt;
                    while let Some(item) = stream.next().await {
                        if let Ok(chunk) = item {
                            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&chunk) {
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
        });

        token_rx
    }
}
