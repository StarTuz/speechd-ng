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
    #[serde(default)]
    pub source: String,  // "passive" or "manual"
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

    /// Passive learning from LLM corrections (lower initial confidence)
    pub fn learn(&self, heard: String, meant: String) {
        if heard.is_empty() || meant.is_empty() || heard == meant {
            return;
        }

        let mut data = self.data.lock().unwrap();
        let entry = data.patterns.entry(heard.to_lowercase()).or_insert(Pattern {
            correction: meant.to_lowercase(),
            count: 0,
            confidence: 0.0,
            source: "passive".to_string(),
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
            entry.source = "passive".to_string();
        }
        
        // Save history (last 100 commands)
        data.command_history.push(meant);
        if data.command_history.len() > 100 {
            data.command_history.remove(0);
        }

        self.save(&data);
    }

    /// Manual learning from explicit user training (higher initial confidence)
    /// Returns true if the pattern was added/updated
    pub fn add_manual_correction(&self, heard: String, meant: String) -> bool {
        if heard.is_empty() || meant.is_empty() || heard == meant {
            return false;
        }

        let mut data = self.data.lock().unwrap();
        let heard_lower = heard.to_lowercase();
        let meant_lower = meant.to_lowercase();
        
        // Manual corrections start with high confidence (0.7) and boost quickly
        let entry = data.patterns.entry(heard_lower.clone()).or_insert(Pattern {
            correction: meant_lower.clone(),
            count: 0,
            confidence: 0.0,
            source: "manual".to_string(),
        });

        if entry.correction == meant_lower {
            entry.count += 1;
            // Manual patterns reach max confidence faster (after 3 confirmations)
            entry.confidence = (0.7 + (entry.count as f32 * 0.1)).min(1.0);
        } else {
            // Override with new correction
            entry.correction = meant_lower;
            entry.count = 1;
            entry.confidence = 0.7;  // Start high for manual
            entry.source = "manual".to_string();
        }

        // Capture values for logging before dropping the mutable reference
        let correction = entry.correction.clone();
        let confidence = entry.confidence;

        self.save(&data);
        println!("Fingerprint: Learned '{}' → '{}' (manual, confidence: {:.0}%)", 
            heard, correction, confidence * 100.0);
        true
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

    /// Get statistics about the fingerprint
    pub fn get_stats(&self) -> (u32, u32, u32) {
        let data = self.data.lock().unwrap();
        let manual_count = data.patterns.values()
            .filter(|p| p.source == "manual")
            .count() as u32;
        let passive_count = data.patterns.values()
            .filter(|p| p.source != "manual")
            .count() as u32;
        let command_count = data.command_history.len() as u32;
        (manual_count, passive_count, command_count)
    }

    /// Get all patterns for debugging/export
    pub fn get_all_patterns(&self) -> Vec<(String, String, f32, String)> {
        let data = self.data.lock().unwrap();
        data.patterns.iter()
            .map(|(heard, p)| (heard.clone(), p.correction.clone(), p.confidence, p.source.clone()))
            .collect()
    }

    fn save(&self, data: &FingerprintData) {
        if let Ok(content) = serde_json::to_string_pretty(data) {
            fs::write(&self.path, content).ok();
        }
    }
}
