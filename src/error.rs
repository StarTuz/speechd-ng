use std::fmt;

/// Unified error type for SpeechD-NG AI and backend operations.
#[derive(Debug, Clone)]
pub enum SpeechdError {
    /// HTTP error from AI prompt request (stores full status string for backward compat)
    AiHttp { status: String },
    /// Connection error from AI prompt request
    AiConnection { detail: String },
    /// Failed to parse AI response JSON
    AiParseFailed,
    /// HTTP error from AI stream request (stores full status string for backward compat)
    StreamHttp { status: String },
    /// Connection error from AI stream request
    StreamConnection { detail: String },
    /// Generic backend error
    Backend { detail: String },
    /// RwLock<Settings> is poisoned
    SettingsPoisoned,
    /// A Mutex is poisoned
    LockPoisoned { resource: String },
}

impl fmt::Display for SpeechdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Backward-compatible Display strings matching the old format!() calls
            SpeechdError::AiHttp { status } => write!(f, "AI Error: HTTP {}", status),
            SpeechdError::AiConnection { detail } => write!(f, "AI Error: {}", detail),
            SpeechdError::AiParseFailed => write!(f, "Failed to parse AI response."),
            SpeechdError::StreamHttp { status } => write!(f, "Stream Error: HTTP {}", status),
            SpeechdError::StreamConnection { detail } => write!(f, "Stream Error: {}", detail),
            SpeechdError::Backend { detail } => write!(f, "Backend Error: {}", detail),
            SpeechdError::SettingsPoisoned => write!(f, "Settings lock poisoned"),
            SpeechdError::LockPoisoned { resource } => {
                write!(f, "Lock poisoned: {}", resource)
            }
        }
    }
}

impl std::error::Error for SpeechdError {}

// Allow converting from String for backward compatibility
impl From<String> for SpeechdError {
    fn from(s: String) -> Self {
        SpeechdError::Backend { detail: s }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_backward_compat() {
        // These strings must match what the old code produced
        assert_eq!(
            SpeechdError::AiHttp {
                status: "500 Internal Server Error".into()
            }
            .to_string(),
            "AI Error: HTTP 500 Internal Server Error"
        );
        assert_eq!(
            SpeechdError::AiConnection {
                detail: "connection refused".into()
            }
            .to_string(),
            "AI Error: connection refused"
        );
        assert_eq!(
            SpeechdError::AiParseFailed.to_string(),
            "Failed to parse AI response."
        );
        assert_eq!(
            SpeechdError::StreamHttp {
                status: "503 Service Unavailable".into()
            }
            .to_string(),
            "Stream Error: HTTP 503 Service Unavailable"
        );
        assert_eq!(
            SpeechdError::StreamConnection {
                detail: "timeout".into()
            }
            .to_string(),
            "Stream Error: timeout"
        );
    }

    #[test]
    fn test_error_from_string() {
        let err: SpeechdError = "something went wrong".to_string().into();
        assert_eq!(err.to_string(), "Backend Error: something went wrong");
    }
}
