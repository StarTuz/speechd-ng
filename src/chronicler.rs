//! The Chronicler: Long-term memory storage for SpeechD-NG
//!
//! With `rag` feature: Uses BERT embeddings for semantic search
//! Without `rag` feature: Uses simple keyword matching (no ML dependencies)

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Memory {
    text: String,
    timestamp_nanos: i64,
}

// ============================================================================
// RAG-enabled Chronicler (with BERT embeddings)
// ============================================================================
#[cfg(feature = "rag")]
mod rag_impl {
    use super::*;
    use candle_core::{Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::bert::{BertModel, Config, DTYPE};
    use hf_hub::{api::sync::Api, Repo};
    use tokenizers::Tokenizer;

    pub struct Chronicler {
        model: BertModel,
        tokenizer: Tokenizer,
        db: sled::Db,
        device: Device,
    }

    impl Chronicler {
        pub fn new(db_path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
            let device = Device::Cpu;

            let api = Api::new()?;
            let repo = api.repo(Repo::model(
                "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            ));

            let config_filename = repo.get("config.json")?;
            let tokenizer_filename = repo.get("tokenizer.json")?;
            let weights_filename = repo.get("model.safetensors")?;

            let config = std::fs::read_to_string(config_filename)?;
            let config: Config = serde_json::from_str(&config)?;
            let tokenizer = Tokenizer::from_file(tokenizer_filename)
                .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;

            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)?
            };
            let model = BertModel::load(vb, &config)?;

            let db = sled::open(db_path)?;

            Ok(Self {
                model,
                tokenizer,
                db,
                device,
            })
        }

        pub fn add_memory(&self, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
            if text.trim().is_empty() {
                return Ok(());
            }

            let embedding = self.get_embedding(text)?;
            let ts_nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
            let memory = Memory {
                text: text.to_string(),
                timestamp_nanos: ts_nanos,
            };

            let vec_data = embedding.to_vec1::<f32>()?;
            let serialized_vec = serde_json::to_vec(&vec_data)?;
            let serialized_mem = serde_json::to_vec(&memory)?;

            let ts_nanos = memory.timestamp_nanos;
            let key = ts_nanos.to_be_bytes();
            self.db.insert(key, serialized_mem)?;
            self.db
                .insert(format!("vec_{}", ts_nanos), serialized_vec)?;

            Ok(())
        }

        fn get_embedding(
            &self,
            text: &str,
        ) -> Result<Tensor, Box<dyn std::error::Error + Send + Sync>> {
            let tokens = self
                .tokenizer
                .encode(text, true)
                .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;
            let token_ids = tokens.get_ids();
            let token_ids = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;
            let token_type_ids = token_ids.zeros_like()?;

            let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;

            let (_n_batch, n_tokens, _hidden_size) = embeddings.dims3()?;
            let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;

            let norm = embeddings.sqr()?.sum_keepdim(1)?.sqrt()?;
            let embeddings = embeddings.broadcast_div(&norm)?;
            Ok(embeddings.squeeze(0)?)
        }

        pub fn search(
            &self,
            query: &str,
            top_k: usize,
        ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
            let query_embedding = self.get_embedding(query)?;
            let query_vec = query_embedding.to_vec1::<f32>()?;

            let mut scores = Vec::new();

            for item in self.db.iter() {
                let (key, value) = item?;
                let key_str = String::from_utf8_lossy(&key);
                if !key_str.starts_with("vec_") {
                    continue;
                }

                let vec_data: Vec<f32> = serde_json::from_slice(&value)?;

                let mut similarity = 0.0;
                for i in 0..query_vec.len().min(vec_data.len()) {
                    similarity += query_vec[i] * vec_data[i];
                }

                let ts_key_str = &key_str[4..];
                if let Ok(ts) = ts_key_str.parse::<i64>() {
                    let ts_bytes = ts.to_be_bytes();
                    if let Some(mem_bytes) = self.db.get(ts_bytes)? {
                        let mem: Memory = serde_json::from_slice(&mem_bytes)?;
                        scores.push((similarity, mem.text));
                    }
                }
            }

            scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            Ok(scores.into_iter().take(top_k).map(|s| s.1).collect())
        }
    }
}

// ============================================================================
// Simple Chronicler (no ML dependencies - keyword matching fallback)
// ============================================================================
#[cfg(not(feature = "rag"))]
mod simple_impl {
    use super::*;

    pub struct Chronicler {
        db: sled::Db,
    }

    impl Chronicler {
        pub fn new(db_path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
            let db = sled::open(db_path)?;
            Ok(Self { db })
        }

        pub fn add_memory(&self, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
            if text.trim().is_empty() {
                return Ok(());
            }

            let ts_nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
            let memory = Memory {
                text: text.to_string(),
                timestamp_nanos: ts_nanos,
            };

            let key = ts_nanos.to_be_bytes();
            let serialized_mem = serde_json::to_vec(&memory)?;
            self.db.insert(key, serialized_mem)?;

            Ok(())
        }

        pub fn search(
            &self,
            query: &str,
            top_k: usize,
        ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
            // Simple keyword matching fallback
            let query_lower = query.to_lowercase();
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();
            let mut matches = Vec::new();

            for item in self.db.iter() {
                let (key, value) = item?;
                let key_bytes: [u8; 8] = match key.as_ref().try_into() {
                    Ok(b) => b,
                    Err(_) => continue,
                };

                // Skip vector keys (only process memory keys)
                if key.len() != 8 {
                    continue;
                }

                let mem: Memory = match serde_json::from_slice(&value) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let text_lower = mem.text.to_lowercase();
                let score: usize = query_words
                    .iter()
                    .filter(|w| text_lower.contains(*w))
                    .count();

                if score > 0 {
                    let ts = i64::from_be_bytes(key_bytes);
                    matches.push((score, ts, mem.text));
                }
            }

            // Sort by score (desc), then by timestamp (desc for recency)
            matches.sort_by(|a, b| {
                b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1))
            });

            Ok(matches.into_iter().take(top_k).map(|m| m.2).collect())
        }
    }
}

// Re-export the appropriate implementation
#[cfg(feature = "rag")]
pub use rag_impl::Chronicler;

#[cfg(not(feature = "rag"))]
pub use simple_impl::Chronicler;
