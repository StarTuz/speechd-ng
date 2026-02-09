use crate::backends::BrainBackend;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};

pub struct FallbackBackend {
    pub primary: Arc<dyn BrainBackend + Send + Sync>,
    pub secondary: Arc<dyn BrainBackend + Send + Sync>,
}

impl FallbackBackend {
    pub fn new(
        primary: Arc<dyn BrainBackend + Send + Sync>,
        secondary: Arc<dyn BrainBackend + Send + Sync>,
    ) -> Self {
        Self { primary, secondary }
    }
}

#[async_trait]
impl BrainBackend for FallbackBackend {
    async fn prompt(&self, system: &str, context: &str, user: &str) -> Result<String, String> {
        match self.primary.prompt(system, context, user).await {
            Ok(res) => Ok(res),
            Err(e) => {
                eprintln!(
                    "Primary AI Backend failed: {}. Falling back to secondary...",
                    e
                );
                self.secondary.prompt(system, context, user).await
            }
        }
    }

    async fn stream(
        &self,
        system: &str,
        context: &str,
        user: &str,
        images: Option<Vec<String>>,
    ) -> Receiver<String> {
        let (tx, rx) = channel::<String>(100);
        let primary = self.primary.clone();
        let secondary = self.secondary.clone();
        let system = system.to_string();
        let context = context.to_string();
        let user = user.to_string();
        let images = images;

        tokio::spawn(async move {
            let mut stream = primary
                .stream(&system, &context, &user, images.clone())
                .await;

            // Wait for first token or error
            if let Some(token) = stream.recv().await {
                if token.contains("AI Error") || token.contains("Stream Error") {
                    eprintln!("Primary AI Stream failed: {}. Falling back...", token);
                    let mut fallback_stream =
                        secondary.stream(&system, &context, &user, images).await;
                    while let Some(t) = fallback_stream.recv().await {
                        let _ = tx.send(t).await;
                    }
                } else {
                    let _ = tx.send(token).await;
                    while let Some(t) = stream.recv().await {
                        let _ = tx.send(t).await;
                    }
                }
            } else {
                // Primary returned nothing immediately, likely a connection error handled in backend
                eprintln!("Primary AI Stream yielded no tokens. Falling back...");
                let mut fallback_stream = secondary.stream(&system, &context, &user, images).await;
                while let Some(t) = fallback_stream.recv().await {
                    let _ = tx.send(t).await;
                }
            }
        });

        rx
    }
}
