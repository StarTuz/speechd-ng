use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Serialize, Deserialize)]
pub struct WyomingEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: Option<serde_json::Value>,
}

pub struct WyomingClient {
    stream: TcpStream,
}

impl WyomingClient {
    pub async fn connect(host: &str, port: u16) -> Result<Self, Box<dyn Error>> {
        let stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
        let mut client = Self { stream };

        // 1. Handshake: Describe
        client.write_event("describe", None).await?;

        // Read response (usually 'info' or 'describe')
        let _info = client.read_event().await?;

        Ok(client)
    }

    pub async fn start_audio(&mut self, rate: u32) -> Result<(), Box<dyn Error>> {
        self.write_event(
            "audio-start",
            Some(json!({
                "rate": rate,
                "width": 2,
                "channels": 1
            })),
        )
        .await
    }

    pub async fn send_chunk(&mut self, audio: &[u8]) -> Result<(), Box<dyn Error>> {
        // Wyoming protocol: event JSON + "\n" + (optional) payload
        // For audio-chunk, the payload is the raw PCM data
        let event_json = json!({
            "type": "audio-chunk",
            "data": {
                "rate": 16000,
                "width": 2,
                "channels": 1,
                "length": audio.len()
            }
        })
        .to_string();

        self.stream.write_all(event_json.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        self.stream.write_all(audio).await?;
        Ok(())
    }

    pub async fn stop_audio(&mut self) -> Result<(), Box<dyn Error>> {
        self.write_event("audio-stop", None).await
    }

    pub async fn wait_for_transcript(&mut self) -> Result<String, Box<dyn Error>> {
        loop {
            let event = self.read_event().await?;
            if event.event_type == "transcript" {
                if let Some(data) = event.data {
                    return Ok(data["text"].as_str().unwrap_or("").to_string());
                }
            }
        }
    }

    async fn write_event(
        &mut self,
        event_type: &str,
        data: Option<serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        let event = json!({
            "type": event_type,
            "data": data
        })
        .to_string();

        self.stream.write_all(event.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        Ok(())
    }

    async fn read_event(&mut self) -> Result<WyomingEvent, Box<dyn Error>> {
        let mut reader = tokio::io::BufReader::new(&mut self.stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let event: WyomingEvent = serde_json::from_str(&line)?;
        Ok(event)
    }
}
