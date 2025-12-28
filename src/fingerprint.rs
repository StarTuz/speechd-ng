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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IgnoredCommand {
    pub heard: String,
    pub timestamp: String,
    #[serde(default)]
    pub context: String,  // Optional context about where it failed
}

#[derive(Clone, Debug)]
pub struct UndoAction {
    pub heard: String,
    pub previous_pattern: Option<Pattern>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct FingerprintData {
    pub patterns: HashMap<String, Pattern>,
    pub command_history: Vec<String>,
    #[serde(default)]
    pub ignored_commands: Vec<IgnoredCommand>,
    
    #[serde(skip)]
    pub undo_stack: Vec<UndoAction>,
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
            self.add_passive_correction(errors[0].to_string(), corrections[0].to_string());
        }
    }

    /// Passive learning from LLM corrections (lower initial confidence)
    pub fn add_passive_correction(&self, heard: String, meant: String) {
        if heard.is_empty() || meant.is_empty() || heard == meant {
            return;
        }

        let initial_confidence = crate::config_loader::SETTINGS.read()
            .map(|s| s.passive_confidence_threshold)
            .unwrap_or(0.1);

        let mut data = self.data.lock().unwrap();
        
        // Undo State
        let heard_lower = heard.to_lowercase();
        let prev = data.patterns.get(&heard_lower).cloned();
        data.undo_stack.push(UndoAction { heard: heard_lower.clone(), previous_pattern: prev });
        if data.undo_stack.len() > 50 { data.undo_stack.remove(0); }

        let entry = data.patterns.entry(heard_lower).or_insert(Pattern {
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
            // If it conflicts, reset to new correction with base confidence
            println!("Fingerprint: Passive correction override: '{}' was '{}', now '{}'", 
                heard, entry.correction, meant);
            entry.correction = meant.to_lowercase();
            entry.count = 1;
            entry.confidence = initial_confidence;
            entry.source = "passive".to_string();
        }
        
        println!("Fingerprint: Passive learned '{}' -> '{}' (conf: {:.2})", heard, meant, entry.confidence);

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
        
        // Undo State
        let prev = data.patterns.get(&heard_lower).cloned();
        data.undo_stack.push(UndoAction { heard: heard_lower.clone(), previous_pattern: prev });
        if data.undo_stack.len() > 50 { data.undo_stack.remove(0); }

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

    /// Rollback the last correction (manual or passive) via undo stack
    pub fn rollback_last_correction(&self) -> bool {
        let mut data = self.data.lock().unwrap();
        if let Some(undo) = data.undo_stack.pop() {
            if let Some(prev) = undo.previous_pattern {
                println!("Fingerprint: Rolling back '{}' to previous state", undo.heard);
                data.patterns.insert(undo.heard, prev);
            } else {
                 println!("Fingerprint: Rolling back '{}' to [deleted]", undo.heard);
                 data.patterns.remove(&undo.heard);
            }
            // Save after rollback
            // (We have to release lock to call self.save if self.save locks? 
            // Check self.save: it takes reference to data, doesn't lock.
            // But wait, self.save takes &FingerprintData. 
            // data is MutexGuard. &*data works.
            if let Ok(content) = serde_json::to_string_pretty(&*data) {
                fs::write(&self.path, content).ok();
            }
            true
        } else {
            false
        }
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

    /// Export fingerprint to a file
    /// Returns true if successful
    pub fn export_to_path(&self, path: &str) -> bool {
        let data = self.data.lock().unwrap();
        match serde_json::to_string_pretty(&*data) {
            Ok(content) => {
                match fs::write(path, content) {
                    Ok(_) => {
                        println!("Fingerprint: Exported {} patterns to {}", data.patterns.len(), path);
                        true
                    }
                    Err(e) => {
                        eprintln!("Fingerprint: Export failed - {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                eprintln!("Fingerprint: Serialization failed - {}", e);
                false
            }
        }
    }

    /// Import fingerprint from a file
    /// If merge=true, merges with existing patterns (existing win on conflict)
    /// If merge=false, replaces current fingerprint entirely
    /// Returns count of patterns after import
    pub fn import_from_path(&self, path: &str, merge: bool) -> u32 {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Fingerprint: Import failed to read file - {}", e);
                return 0;
            }
        };

        let imported: FingerprintData = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Fingerprint: Import failed to parse JSON - {}", e);
                return 0;
            }
        };

        let mut data = self.data.lock().unwrap();
        
        if merge {
            // Merge: imported patterns fill in gaps, don't overwrite existing
            let mut added = 0u32;
            for (heard, pattern) in imported.patterns {
                if !data.patterns.contains_key(&heard) {
                    data.patterns.insert(heard, pattern);
                    added += 1;
                }
            }
            println!("Fingerprint: Merged {} new patterns from {}", added, path);
        } else {
            // Replace: overwrite everything
            *data = imported;
            println!("Fingerprint: Replaced with {} patterns from {}", data.patterns.len(), path);
        }

        self.save(&data);
        data.patterns.len() as u32
    }

    /// Get the path to the fingerprint file
    pub fn get_path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    // ========== Phase 11: Ignored Commands ==========

    /// Add a command that couldn't be understood (for later correction)
    pub fn add_ignored_command(&self, heard: &str, context: &str) {
        if heard.is_empty() {
            return;
        }

        let mut data = self.data.lock().unwrap();
        
        // Don't add duplicates (check last 10)
        let recent: Vec<_> = data.ignored_commands.iter().rev().take(10).collect();
        if recent.iter().any(|c| c.heard == heard) {
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        data.ignored_commands.push(IgnoredCommand {
            heard: heard.to_string(),
            timestamp,
            context: context.to_string(),
        });

        // Cap at 50 most recent
        if data.ignored_commands.len() > 50 {
            data.ignored_commands.remove(0);
        }

        self.save(&data);
        println!("Fingerprint: Added ignored command '{}' (context: {})", heard, context);
    }

    /// Get all ignored commands for review
    pub fn get_ignored_commands(&self) -> Vec<(String, String, String)> {
        let data = self.data.lock().unwrap();
        data.ignored_commands.iter()
            .map(|c| (c.heard.clone(), c.timestamp.clone(), c.context.clone()))
            .collect()
    }

    /// Clear all ignored commands
    pub fn clear_ignored_commands(&self) -> u32 {
        let mut data = self.data.lock().unwrap();
        let count = data.ignored_commands.len() as u32;
        data.ignored_commands.clear();
        self.save(&data);
        println!("Fingerprint: Cleared {} ignored commands", count);
        count
    }

    /// Correct an ignored command - adds it as a pattern and removes from ignored
    /// Returns true if the command was found and corrected
    pub fn correct_ignored_command(&self, heard: &str, meant: &str) -> bool {
        let mut data = self.data.lock().unwrap();
        
        // Find and remove from ignored list
        let original_len = data.ignored_commands.len();
        data.ignored_commands.retain(|c| c.heard.to_lowercase() != heard.to_lowercase());
        let removed = original_len != data.ignored_commands.len();

        if removed {
            // Drop the lock before calling add_manual_correction
            drop(data);
            
            // Add as a manual correction
            self.add_manual_correction(heard.to_string(), meant.to_string());
            println!("Fingerprint: Corrected ignored '{}' -> '{}'", heard, meant);
            true
        } else {
            println!("Fingerprint: Ignored command '{}' not found", heard);
            false
        }
    }

    fn save(&self, data: &FingerprintData) {
        if let Ok(content) = serde_json::to_string_pretty(data) {
            fs::write(&self.path, content).ok();
        }
    }
}
