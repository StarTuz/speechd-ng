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
    /// Limits per minute for each type (wrapped so they can be updated at runtime)
    limits: Mutex<(f32, f32, f32, f32)>, // (tts, ai, audio, listen)
}

impl RateLimiter {
    pub fn new(tts: u32, ai: u32, audio: u32, listen: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            limits: Mutex::new((tts as f32, ai as f32, audio as f32, listen as f32)),
        }
    }

    /// Update rate limits at runtime (e.g. after config reload).
    /// Existing in-flight buckets keep their current token counts but will
    /// refill at the new rate from the next check onward.
    pub fn update_limits(&self, tts: u32, ai: u32, audio: u32, listen: u32) {
        let mut l = self.limits.lock().unwrap_or_else(|p| p.into_inner());
        *l = (tts as f32, ai as f32, audio as f32, listen as f32);
        // Clear existing buckets so new max_tokens takes effect immediately.
        self.buckets
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }

    fn limits_snapshot(&self) -> (f32, f32, f32, f32) {
        *self.limits.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Check if request is allowed, consuming a token if so
    /// Returns true if allowed, false if rate limited
    pub fn check(&self, sender: &str, limit_type: LimitType) -> bool {
        let (tts, ai, audio, listen) = self.limits_snapshot();
        let limit = match limit_type {
            LimitType::Tts => tts,
            LimitType::Ai => ai,
            LimitType::Audio => audio,
            LimitType::Listen => listen,
        };

        // Use burst size = 1 minute worth
        let max_tokens = limit;

        let mut buckets = self.buckets.lock().unwrap_or_else(|p| p.into_inner());
        let key = (sender.to_string(), limit_type);

        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(max_tokens, limit));

        bucket.try_consume(1.0)
    }

    /// Get remaining tokens for a sender/type (for debugging/info)
    #[allow(dead_code)]
    pub fn remaining(&self, sender: &str, limit_type: LimitType) -> f32 {
        let (tts, ai, audio, listen) = self.limits_snapshot();
        let default = match limit_type {
            LimitType::Tts => tts,
            LimitType::Ai => ai,
            LimitType::Audio => audio,
            LimitType::Listen => listen,
        };
        let buckets = self.buckets.lock().unwrap_or_else(|p| p.into_inner());
        buckets
            .get(&(sender.to_string(), limit_type))
            .map(|b| b.tokens)
            .unwrap_or(default)
    }

    /// Clean up old entries (senders not seen recently)
    pub fn cleanup(&self, max_age_secs: u64) {
        let mut buckets = self.buckets.lock().unwrap_or_else(|p| p.into_inner());
        let now = Instant::now();

        buckets.retain(|_, bucket| now.duration_since(bucket.last_update).as_secs() < max_age_secs);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(30, 10, 20, 30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_exhaustion() {
        let limiter = RateLimiter::new(2, 10, 20, 30);
        assert!(limiter.check("sender1", LimitType::Tts));
        assert!(limiter.check("sender1", LimitType::Tts));
        // Third call should be rate limited
        assert!(!limiter.check("sender1", LimitType::Tts));
    }

    #[test]
    fn test_per_sender_independence() {
        let limiter = RateLimiter::new(1, 10, 20, 30);
        assert!(limiter.check("alice", LimitType::Tts));
        assert!(!limiter.check("alice", LimitType::Tts));
        // Different sender should still have tokens
        assert!(limiter.check("bob", LimitType::Tts));
    }

    #[test]
    fn test_per_type_independence() {
        let limiter = RateLimiter::new(1, 1, 1, 1);
        assert!(limiter.check("sender1", LimitType::Tts));
        assert!(!limiter.check("sender1", LimitType::Tts));
        // Different type should still have tokens
        assert!(limiter.check("sender1", LimitType::Ai));
        assert!(limiter.check("sender1", LimitType::Audio));
        assert!(limiter.check("sender1", LimitType::Listen));
    }

    #[test]
    fn test_cleanup_removes_old_entries() {
        let limiter = RateLimiter::new(10, 10, 10, 10);
        limiter.check("old_sender", LimitType::Tts);
        // Cleanup with max_age_secs=0 should remove everything
        limiter.cleanup(0);
        let buckets = limiter.buckets.lock().unwrap();
        assert!(buckets.is_empty());
    }

    #[test]
    fn test_cleanup_retains_recent() {
        let limiter = RateLimiter::new(10, 10, 10, 10);
        limiter.check("recent_sender", LimitType::Tts);
        // Cleanup with large max_age should retain
        limiter.cleanup(3600);
        let buckets = limiter.buckets.lock().unwrap();
        assert!(!buckets.is_empty());
    }

    #[test]
    fn test_remaining_count() {
        let limiter = RateLimiter::new(5, 10, 20, 30);
        limiter.check("sender1", LimitType::Tts);
        let remaining = limiter.remaining("sender1", LimitType::Tts);
        // Should be approximately 4.0 (5 - 1, possibly slightly more from refill)
        assert!(remaining >= 3.9 && remaining <= 4.1);
    }
}
