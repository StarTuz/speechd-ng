use crate::backends::BrainBackend;
use crate::error::SpeechdError;
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
    async fn prompt(
        &self,
        system: &str,
        context: &str,
        user: &str,
    ) -> Result<String, SpeechdError> {
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
    ) -> Receiver<Result<String, SpeechdError>> {
        let (tx, rx) = channel::<Result<String, SpeechdError>>(100);
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

            // Wait for first token or error — type-safe detection
            if let Some(result) = stream.recv().await {
                match result {
                    Err(e) => {
                        eprintln!("Primary AI Stream failed: {}. Falling back...", e);
                        let mut fallback_stream =
                            secondary.stream(&system, &context, &user, images).await;
                        while let Some(t) = fallback_stream.recv().await {
                            let _ = tx.send(t).await;
                        }
                    }
                    Ok(token) => {
                        let _ = tx.send(Ok(token)).await;
                        while let Some(t) = stream.recv().await {
                            let _ = tx.send(t).await;
                        }
                    }
                }
            } else {
                // Primary returned nothing, likely a connection error
                eprintln!("Primary AI Stream yielded no tokens. Falling back...");
                let mut fallback_stream =
                    secondary.stream(&system, &context, &user, images).await;
                while let Some(t) = fallback_stream.recv().await {
                    let _ = tx.send(t).await;
                }
            }
        });

        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SuccessBackend {
        response: String,
    }

    #[async_trait]
    impl BrainBackend for SuccessBackend {
        async fn prompt(
            &self,
            _s: &str,
            _c: &str,
            _u: &str,
        ) -> Result<String, SpeechdError> {
            Ok(self.response.clone())
        }
        async fn stream(
            &self,
            _s: &str,
            _c: &str,
            _u: &str,
            _images: Option<Vec<String>>,
        ) -> Receiver<Result<String, SpeechdError>> {
            let (tx, rx) = channel(10);
            let resp = self.response.clone();
            tokio::spawn(async move {
                let _ = tx.send(Ok(resp)).await;
            });
            rx
        }
    }

    struct FailBackend;

    #[async_trait]
    impl BrainBackend for FailBackend {
        async fn prompt(
            &self,
            _s: &str,
            _c: &str,
            _u: &str,
        ) -> Result<String, SpeechdError> {
            Err(SpeechdError::AiConnection {
                detail: "connection refused".into(),
            })
        }
        async fn stream(
            &self,
            _s: &str,
            _c: &str,
            _u: &str,
            _images: Option<Vec<String>>,
        ) -> Receiver<Result<String, SpeechdError>> {
            let (tx, rx) = channel(10);
            tokio::spawn(async move {
                let _ = tx
                    .send(Err(SpeechdError::StreamConnection {
                        detail: "connection refused".into(),
                    }))
                    .await;
            });
            rx
        }
    }

    #[tokio::test]
    async fn test_primary_success_passthrough() {
        let fb = FallbackBackend::new(
            Arc::new(SuccessBackend {
                response: "primary OK".into(),
            }),
            Arc::new(SuccessBackend {
                response: "secondary OK".into(),
            }),
        );
        let result = fb.prompt("s", "c", "u").await;
        assert_eq!(result.unwrap(), "primary OK");
    }

    #[tokio::test]
    async fn test_primary_error_triggers_secondary() {
        let fb = FallbackBackend::new(
            Arc::new(FailBackend),
            Arc::new(SuccessBackend {
                response: "secondary OK".into(),
            }),
        );
        let result = fb.prompt("s", "c", "u").await;
        assert_eq!(result.unwrap(), "secondary OK");
    }

    #[tokio::test]
    async fn test_stream_primary_success() {
        let fb = FallbackBackend::new(
            Arc::new(SuccessBackend {
                response: "streamed".into(),
            }),
            Arc::new(SuccessBackend {
                response: "fallback".into(),
            }),
        );
        let mut rx = fb.stream("s", "c", "u", None).await;
        let result = rx.recv().await.unwrap();
        assert_eq!(result.unwrap(), "streamed");
    }

    #[tokio::test]
    async fn test_stream_error_triggers_fallback() {
        let fb = FallbackBackend::new(
            Arc::new(FailBackend),
            Arc::new(SuccessBackend {
                response: "fallback OK".into(),
            }),
        );
        let mut rx = fb.stream("s", "c", "u", None).await;
        let mut tokens = Vec::new();
        while let Some(t) = rx.recv().await {
            if let Ok(token) = t {
                tokens.push(token);
            }
        }
        assert!(tokens.contains(&"fallback OK".to_string()));
    }
}
