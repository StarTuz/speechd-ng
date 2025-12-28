use super::{SpeechBackend, Voice};

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

    fn list_voices(&self) -> Result<Vec<Voice>> {
        use std::process::Command;
        use crate::backends::Voice;

        let output = Command::new("espeak-ng")
            .arg("--voices")
            .output()?;
        
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
                    
                    if let (Some(lang), Some(name), Some(id)) = (parts.get(1), parts.get(3), parts.get(4)) {
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
        use std::process::{Stdio, Command};
        use wait_timeout::ChildExt;
        use std::time::Duration;

        let mut cmd = Command::new("espeak-ng");
        cmd.arg("--stdout");
        
        if let Some(v) = voice {
            cmd.arg("-v").arg(v);
        }
        
        cmd.arg(text)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        
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
