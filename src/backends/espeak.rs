use super::{SpeechBackend, Voice};

use std::io::{Error, ErrorKind, Result};

pub struct EspeakBackend;

impl EspeakBackend {
    pub fn new() -> Self {
        Self
    }
}

impl SpeechBackend for EspeakBackend {
    fn list_voices(&self) -> Result<Vec<Voice>> {
        use crate::backends::Voice;
        use std::process::Command;

        let output = Command::new("espeak-ng").arg("--voices").output()?;

        let mut voices = Vec::new();

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Skip header line
            for line in stdout.lines().skip(1) {
                // espeak output format is fixed width tables, usually:
                // Pty Language Age/Gender VoiceName       File        Other
                // We do a rough split.
                // Better approach: use `espeak-ng --voices=variant` or JSON if available,
                // but espeak-ng usually just outputs text table.
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    // This is heuristic parsing.
                    // parts[1] is typically language (en-us)
                    // parts[3] is typically name?
                    // parts[4] is file/id?

                    // Example:
                    //  5  en-us          M  en-us           en-us       (en 5)
                    // Let's assume:
                    // Language = parts[1]
                    // ID = parts[4] (the file name to pass to -v)
                    // Name = parts[3] ??

                    if let (Some(lang), Some(name), Some(id)) =
                        (parts.get(1), parts.get(3), parts.get(4))
                    {
                        voices.push(Voice {
                            id: id.to_string(),
                            name: name.to_string(),
                            language: lang.to_string(),
                        });
                    }
                }
            }
        }
        Ok(voices)
    }

    fn synthesize(&self, text: &str, voice: Option<&str>) -> Result<Vec<u8>> {
        use std::io::Read;
        use std::process::{Command, Stdio};
        use std::time::Duration;
        use wait_timeout::ChildExt;

        let mut cmd = Command::new("espeak-ng");
        cmd.arg("--stdout");

        if let Some(v) = voice {
            cmd.arg("-v").arg(v);
        }

        // Use -- to separate flags from text
        cmd.arg("--")
            .arg(text)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        // Capture stdout before waiting (wait_timeout consumes the child)
        let mut stdout_data = Vec::new();
        if let Some(ref mut stdout) = child.stdout.take() {
            // Read in a separate scope to avoid holding the borrow
            stdout.read_to_end(&mut stdout_data)?;
        }

        // Wait for process to complete (with timeout)
        match child.wait_timeout(Duration::from_secs(5))? {
            Some(status) => {
                if status.success() {
                    Ok(stdout_data)
                } else {
                    // Read stderr for error message
                    let mut stderr_data = Vec::new();
                    if let Some(ref mut stderr) = child.stderr.take() {
                        let _ = stderr.read_to_end(&mut stderr_data);
                    }
                    let err_msg = String::from_utf8_lossy(&stderr_data);
                    Err(Error::new(
                        ErrorKind::Other,
                        format!("espeak error: {}", err_msg),
                    ))
                }
            }
            None => {
                // Timeout occurred, kill the process
                let _ = child.kill();
                let _ = child.wait();
                Err(Error::new(
                    ErrorKind::TimedOut,
                    "Backend timed out after 5s",
                ))
            }
        }
    }
}
