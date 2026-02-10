/// Parsed audio sink from wpctl output
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSink {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

/// Parse wpctl status output to extract audio sinks.
/// Pure function — highly testable.
pub fn parse_wpctl_sinks(stdout: &str) -> Vec<ParsedSink> {
    let mut sinks = Vec::new();
    let mut in_sinks_section = false;

    for line in stdout.lines() {
        if line.contains("Sinks:") && !line.contains("Sources:") {
            in_sinks_section = true;
            continue;
        }
        if in_sinks_section {
            if line.contains("Sources:")
                || line.contains("Streams:")
                || line.contains("Filters:")
            {
                break;
            }

            if !line.contains("[vol:") && !line.contains('.') {
                continue;
            }

            let is_default = line.contains('*');
            let cleaned: String = line
                .chars()
                .filter(|c| !['│', '├', '└', '─', '┬', '┤', '┴', '┼'].contains(c))
                .collect();
            let trimmed = cleaned.trim().trim_start_matches('*').trim();

            if let Some(dot_pos) = trimmed.find('.') {
                if let Ok(id) = trimmed[..dot_pos].trim().parse::<u32>() {
                    let rest = trimmed[dot_pos + 1..].trim();
                    let name = if let Some(vol_pos) = rest.find("[vol:") {
                        rest[..vol_pos].trim()
                    } else {
                        rest
                    };

                    if !name.is_empty() {
                        sinks.push(ParsedSink {
                            id,
                            name: name.to_string(),
                            description: name.to_string(),
                            is_default,
                        });
                    }
                }
            }
        }
    }
    sinks
}

/// List sinks by running wpctl status.
pub fn list_sinks() -> Vec<(u32, String, String, bool)> {
    match std::process::Command::new("/usr/bin/wpctl")
        .arg("status")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stdout.is_empty() {
                eprintln!("ListSinks: wpctl returned empty stdout, stderr: {}", stderr);
            }

            parse_wpctl_sinks(&stdout)
                .into_iter()
                .map(|s| (s.id, s.name, s.description, s.is_default))
                .collect()
        }
        Err(e) => {
            eprintln!("Failed to run wpctl: {}", e);
            Vec::new()
        }
    }
}

/// Get the current default sink ID by running wpctl inspect.
pub fn get_current_default_sink_id() -> Option<u32> {
    std::process::Command::new("/usr/bin/wpctl")
        .args(["inspect", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .ok()
        .and_then(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout
                .lines()
                .find(|l| l.trim().starts_with("id"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|s| s.trim_matches(',').parse::<u32>().ok())
        })
}

/// Set the default audio sink.
pub fn set_default_sink(device_id: u32) -> bool {
    std::process::Command::new("/usr/bin/wpctl")
        .args(["set-default", &device_id.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPICAL_WPCTL: &str = r#"
PipeWire 'pipewire-0' [1.2.7, user@host, cookie:12345]
 └─ Clients:
        31. PipeWire                            [pipewire, 1234]

Audio
 ├─ Devices:
 │      40. alsa_card.pci-0000_00_1f.3          [alsa]
 │
 ├─ Sinks:
 │      42. Built-in Audio Analog Stereo        [vol: 0.50]
 │  *  46. USB Speaker                          [vol: 0.75]
 │
 ├─ Sources:
 │      43. Built-in Audio Analog Stereo        [vol: 1.00]
"#;

    #[test]
    fn test_parse_typical_wpctl_output() {
        let sinks = parse_wpctl_sinks(TYPICAL_WPCTL);
        assert_eq!(sinks.len(), 2);
        assert_eq!(sinks[0].id, 42);
        assert_eq!(sinks[0].name, "Built-in Audio Analog Stereo");
        assert!(!sinks[0].is_default);
        assert_eq!(sinks[1].id, 46);
        assert_eq!(sinks[1].name, "USB Speaker");
        assert!(sinks[1].is_default);
    }

    #[test]
    fn test_parse_default_marker() {
        let sinks = parse_wpctl_sinks(TYPICAL_WPCTL);
        let default_sinks: Vec<_> = sinks.iter().filter(|s| s.is_default).collect();
        assert_eq!(default_sinks.len(), 1);
        assert_eq!(default_sinks[0].name, "USB Speaker");
    }

    #[test]
    fn test_parse_empty_output() {
        let sinks = parse_wpctl_sinks("");
        assert!(sinks.is_empty());
    }

    #[test]
    fn test_parse_no_sinks_section() {
        let output = "Audio\n ├─ Sources:\n │      43. Mic [vol: 1.00]\n";
        let sinks = parse_wpctl_sinks(output);
        assert!(sinks.is_empty());
    }
}
