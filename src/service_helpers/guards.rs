use crate::rate_limiter::{LimitType, RateLimiter};
use crate::security::SecurityAgent;
use std::sync::Arc;
use zbus::message::Header;
use zbus::Connection;

/// Check Polkit authorization for a D-Bus caller.
/// Returns Ok(()) if authorized or if sender cannot be determined (permissive fallback).
pub async fn check_polkit(
    conn: &Connection,
    header: &Header<'_>,
    action: &str,
) -> zbus::fdo::Result<()> {
    if let Some(sender) = header.sender() {
        if let Ok(pid) = SecurityAgent::get_sender_pid(conn, sender.as_str()).await {
            if let Err(e) = SecurityAgent::check_permission_polkit(pid, action).await {
                eprintln!("Access Denied: {}", e);
                return Err(zbus::fdo::Error::AccessDenied(format!(
                    "Polkit denied: {}",
                    e
                )));
            }
        }
    }
    Ok(())
}

/// Check rate limit for a D-Bus caller.
/// Returns Ok(()) if allowed, Err(Failed) if rate limited.
pub fn check_rate_limit(
    header: &Header<'_>,
    rate_limiter: &Arc<RateLimiter>,
    limit_type: LimitType,
    label: &str,
) -> zbus::fdo::Result<()> {
    if let Some(sender) = header.sender() {
        if !rate_limiter.check(sender.as_str(), limit_type) {
            println!("Rate limited: {} for sender {}", label, sender);
            return Err(zbus::fdo::Error::Failed("Rate limited".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limiter::RateLimiter;

    #[test]
    fn test_rate_limit_allows_within_budget() {
        let limiter = Arc::new(RateLimiter::new(10, 10, 10, 10));
        // No header sender means no rate limit check — always passes
        // We can't easily construct a Header in tests, so we test the limiter directly
        assert!(limiter.check("test-sender", LimitType::Tts));
    }

    #[test]
    fn test_rate_limit_blocks_when_exhausted() {
        let limiter = Arc::new(RateLimiter::new(1, 1, 1, 1));
        assert!(limiter.check("test-sender", LimitType::Ai));
        assert!(!limiter.check("test-sender", LimitType::Ai));
    }
}
