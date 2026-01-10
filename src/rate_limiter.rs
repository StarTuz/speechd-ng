use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Token bucket for rate limiting
struct TokenBucket {
    tokens: f32,
    max_tokens: f32,
    refill_rate: f32, // tokens per second
    last_update: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f32, tokens_per_minute: f32) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate: tokens_per_minute / 60.0, // Convert to per second
            last_update: Instant::now(),
        }
    }

    fn try_consume(&mut self, tokens: f32) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f32();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_update = now;
    }
}

/// Rate limit types for different method categories
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum LimitType {
    Tts,    // Speak, SpeakVoice, SpeakChannel
    Ai,     // Think
    Audio,  // PlayAudio, PlayAudioChannel
    Listen, // Listen, ListenVad
}

/// Per-sender rate limiter
pub struct RateLimiter {
    /// Map of (sender, limit_type) -> TokenBucket
    buckets: Mutex<HashMap<(String, LimitType), TokenBucket>>,
    /// Limits per minute for each type
    tts_per_minute: f32,
    ai_per_minute: f32,
    audio_per_minute: f32,
    listen_per_minute: f32,
}

impl RateLimiter {
    pub fn new(tts: u32, ai: u32, audio: u32, listen: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            tts_per_minute: tts as f32,
            ai_per_minute: ai as f32,
            audio_per_minute: audio as f32,
            listen_per_minute: listen as f32,
        }
    }

    /// Check if request is allowed, consuming a token if so
    /// Returns true if allowed, false if rate limited
    pub fn check(&self, sender: &str, limit_type: LimitType) -> bool {
        let limit = match limit_type {
            LimitType::Tts => self.tts_per_minute,
            LimitType::Ai => self.ai_per_minute,
            LimitType::Audio => self.audio_per_minute,
            LimitType::Listen => self.listen_per_minute,
        };

        // Use burst size = 1 minute worth
        let max_tokens = limit;

        let mut buckets = self.buckets.lock().unwrap();
        let key = (sender.to_string(), limit_type);

        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(max_tokens, limit));

        bucket.try_consume(1.0)
    }

    /// Get remaining tokens for a sender/type (for debugging/info)
    #[allow(dead_code)]
    pub fn remaining(&self, sender: &str, limit_type: LimitType) -> f32 {
        let buckets = self.buckets.lock().unwrap();
        let key = (sender.to_string(), limit_type);

        buckets
            .get(&key)
            .map(|b| b.tokens)
            .unwrap_or(self.tts_per_minute)
    }

    /// Clean up old entries (senders not seen recently)
    pub fn cleanup(&self, max_age_secs: u64) {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();

        buckets.retain(|_, bucket| now.duration_since(bucket.last_update).as_secs() < max_age_secs);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(30, 10, 20, 30) // Default limits per minute
    }
}
