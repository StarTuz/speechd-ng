use super::SpeechBackend;

use std::io::{Result, Error, ErrorKind};

pub struct EspeakBackend;

impl EspeakBackend {
    pub fn new() -> Self {
        Self
    }
}

impl SpeechBackend for EspeakBackend {
    fn id(&self) -> &'static str {
        "espeak-ng"
    }

    fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        use std::process::{Stdio, Command};
        use wait_timeout::ChildExt;
        use std::time::Duration;

        let mut child = Command::new("espeak-ng")
            .arg("--stdout")
            .arg(text)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait for 5 seconds max
        match child.wait_timeout(Duration::from_secs(5))? {
            Some(status) => {
                if status.success() {
                    let output = child.wait_with_output()?;
                    Ok(output.stdout)
                } else {
                    let output = child.wait_with_output()?;
                    let err_msg = String::from_utf8_lossy(&output.stderr);
                    Err(Error::new(ErrorKind::Other, format!("espeak error: {}", err_msg)))
                }
            },
            None => {
                // Timeout occurred, kill the process
                let _ = child.kill();
                let _ = child.wait();
                Err(Error::new(ErrorKind::TimedOut, "Backend timed out after 5s"))
            }
        }
    }
}
