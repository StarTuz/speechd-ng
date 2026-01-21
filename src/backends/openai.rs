use crate::backends::BrainBackend;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc::{channel, Receiver};

pub struct OpenAIBackend {
    pub url: String,
    pub model: String,
    pub client: Client,
}

impl OpenAIBackend {
    pub fn new(url: String, model: String, client: Client) -> Self {
        Self { url, model, client }
    }
}

#[async_trait]
impl BrainBackend for OpenAIBackend {
    async fn prompt(&self, system: &str, context: &str, user: &str) -> Result<String, String> {
        let payload = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": format!("Context:\n{}\n\nUser: {}", context, user)}
            ],
            "stream": false
        });

        match self
            .client
            .post(format!(
                "{}/v1/chat/completions",
                self.url.trim_end_matches('/')
            ))
            .json(&payload)
            .send()
            .await
        {
            Ok(res) => {
                if !res.status().is_success() {
                    return Err(format!("AI Error: HTTP {}", res.status()));
                }
                if let Ok(json) = res.json::<serde_json::Value>().await {
                    Ok(json["choices"][0]["message"]["content"]
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
        _images: Option<Vec<String>>,
    ) -> Receiver<String> {
        let (token_tx, token_rx) = channel::<String>(100);
        let url = format!("{}/v1/chat/completions", self.url.trim_end_matches('/'));
        let client = self.client.clone();
        let payload = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": format!("Context:\n{}\n\nUser: {}", context, user)}
            ],
            "stream": true
        });

        tokio::spawn(async move {
            let res = client.post(url).json(&payload).send().await;

            match res {
                Ok(response) => {
                    if !response.status().is_success() {
                        let _ = token_tx
                            .send(format!("Stream Error: HTTP {}", response.status()))
                            .await;
                        return;
                    }
                    let mut stream = response.bytes_stream();
                    use futures_util::StreamExt;
                    while let Some(item) = stream.next().await {
                        if let Ok(chunk) = item {
                            let text = String::from_utf8_lossy(&chunk);
                            for line in text.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        break;
                                    }
                                    if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(data)
                                    {
                                        if let Some(token) =
                                            json["choices"][0]["delta"]["content"].as_str()
                                        {
                                            let _ = token_tx.send(token.to_string()).await;
                                        }
                                    }
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
