use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pattern {
    pub correction: String,
    pub count: u32,
    pub confidence: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct FingerprintData {
    pub patterns: HashMap<String, Pattern>,
    pub command_history: Vec<String>,
}

#[derive(Clone)]
pub struct Fingerprint {
    path: PathBuf,
    data: Arc<Mutex<FingerprintData>>,
}

impl Fingerprint {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let data_dir = home.join(".local/share/speechd-ng");
        fs::create_dir_all(&data_dir).ok();
        let path = data_dir.join("fingerprint.json");

        let data = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            FingerprintData::default()
        };

        Self {
            path,
            data: Arc::new(Mutex::new(data)),
        }
    }

    pub fn passive_learn(&self, asr_heard: &str, final_text: &str) {
        let asr_words: HashSet<&str> = asr_heard.split_whitespace().collect();
        let final_words: HashSet<&str> = final_text.split_whitespace().collect();

        // Find words unique to ASR (errors)
        let errors: Vec<&&str> = asr_words.difference(&final_words).collect();
        // Find words unique to Final (corrections)
        let corrections: Vec<&&str> = final_words.difference(&asr_words).collect();

        // Simple case: one word was corrected
        if errors.len() == 1 && corrections.len() == 1 {
            self.learn(errors[0].to_string(), corrections[0].to_string());
        }
    }

    pub fn learn(&self, heard: String, meant: String) {
        if heard.is_empty() || meant.is_empty() || heard == meant {
            return;
        }

        let mut data = self.data.lock().unwrap();
        let entry = data.patterns.entry(heard.to_lowercase()).or_insert(Pattern {
            correction: meant.to_lowercase(),
            count: 0,
            confidence: 0.0,
        });

        if entry.correction == meant.to_lowercase() {
            entry.count += 1;
            // Confidence increases with frequency, maxing at 1.0 after 10 successes
            entry.confidence = (entry.count as f32 / 10.0).min(1.0);
        } else {
            // If it conflicts, reset to new correction
            entry.correction = meant.to_lowercase();
            entry.count = 1;
            entry.confidence = 0.1;
        }
        
        // Save history (last 100 commands)
        data.command_history.push(meant);
        if data.command_history.len() > 100 {
            data.command_history.remove(0);
        }

        self.save(&data);
    }

    pub fn get_corrections_prompt(&self, text: &str) -> String {
        let data = self.data.lock().unwrap();
        let mut prompt_parts = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        let mut matched = HashSet::new();

        for word in words {
            let word_lower = word.to_lowercase();
            if matched.contains(&word_lower) {
                continue;
            }

            if let Some(pattern) = data.patterns.get(&word_lower) {
                if pattern.confidence > 0.3 {
                    prompt_parts.push(format!("- \"{}\" likely means \"{}\" (confidence: {:.0}%)", 
                        word, pattern.correction, pattern.confidence * 100.0));
                    matched.insert(word_lower);
                }
            }
        }

        if prompt_parts.is_empty() {
            String::new()
        } else {
            format!("\nPERSONALIZED CORRECTIONS (learned from this user's voice):\n{}\n", 
                prompt_parts.join("\n"))
        }
    }

    fn save(&self, data: &FingerprintData) {
        if let Ok(content) = serde_json::to_string_pretty(data) {
            fs::write(&self.path, content).ok();
        }
    }
}
