# SpeechD-NG D-Bus API Reference

This document provides a complete reference for GUI developers and integrators who want to interact with the SpeechD-NG daemon via D-Bus.

## Connection Details

| Property | Value |
|----------|-------|
| **Bus** | Session Bus |
| **Service Name** | `org.speech.Service` |
| **Object Path** | `/org/speech/Service` |
| **Interface** | `org.speech.Service` |

### Quick Test

```bash
# Verify service is running
busctl --user introspect org.speech.Service /org/speech/Service
```

## Core / Diagnostic Methods

### `Ping() → String` (Phase 14)

Diagnostic method to verify D-Bus connectivity.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Ping
```

**Returns:** `"pong"`

---

## Core TTS Methods

### `Speak(text: String)`

Speak text using the default voice and backend.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Speak s "Hello world"
```

**Python Example:**
```python
import dbus

bus = dbus.SessionBus()
service = bus.get_object('org.speech.Service', '/org/speech/Service')
iface = dbus.Interface(service, 'org.speech.Service')
iface.Speak("Hello world")
```

---

### `SpeakVoice(text: String, voice: String)`

Speak text using a specific voice.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service SpeakVoice ss "Hello" "en_US-amy-medium"
```

**Parameters:**
- `text`: Text to speak
- `voice`: Voice ID (e.g., `en_US-lessac-medium`, `en_GB-jenny_dioco-medium`)

---

### `ListVoices() → Vec<(id: String, name: String)>`

List all locally installed voices.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListVoices
```

**Returns:** Array of (voice_id, display_name) tuples.

---

### `ListDownloadableVoices() → Vec<(id: String, description: String)>`

List voices available for download from Hugging Face.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListDownloadableVoices
```

---

### `DownloadVoice(voice_id: String) → String`

Download a Piper neural voice model.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service DownloadVoice s "piper:en_US-amy-low"
```

**Returns:** `"Success"` or `"Error: <message>"`

---

## AI / LLM Methods

### `Think(query: String) → String`

Ask the AI cortex a question about recent speech context.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Think s "What was just said about the meeting?"
```

**Returns:** AI-generated response based on speech history.

---

### `Listen() → String`

Record audio and transcribe it using STT (fixed 4-second duration).

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Listen
```

**Returns:** Transcribed text.

---

### `ListenVad() → String` (Phase 12)

Record audio with Voice Activity Detection - waits for speech, records until silence.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListenVad
```

**Returns:** Transcribed text.

**Notes:**
- Waits for speech to begin (doesn't record initial silence)
- Automatically ends when user stops speaking
- Uses configurable energy thresholds
- More natural than fixed-duration recording

**Configuration** (`~/.config/speechd-ng/Speech.toml`):
```toml
vad_speech_threshold = 500      # Energy level to detect speech start
vad_silence_threshold = 400     # Energy level to detect silence
vad_silence_duration_ms = 1500  # Silence duration before ending
vad_max_duration_ms = 15000     # Maximum recording length
```

---

## Voice Training Methods (Phase 9)

### `AddCorrection(heard: String, meant: String) → bool`

Add a manual voice correction pattern. Use when you know what ASR mishears.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service AddCorrection ss "ever" "abba"
```

**Parameters:**
- `heard`: What ASR incorrectly transcribes
- `meant`: What the user actually said

**Returns:** `true` if the pattern was added.

- Confidence increases faster with repeated confirmations

---

### `RollbackLastCorrection() → bool`

Undo the most recent voice pattern learning (manual or passive).

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service RollbackLastCorrection
```

**Returns:** `true` if a correction was rolled back.

---

### `TrainWord(expected: String, duration_secs: u32) → (heard: String, success: bool)`

Record audio and learn what ASR hears for a specific word.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service TrainWord su "beethoven" 3
```

**Parameters:**
- `expected`: What the user intends to say
- `duration_secs`: Recording duration in seconds

**Returns:** 
- `heard`: What ASR transcribed
- `success`: Whether the pattern was learned

**Notes:**
- Speaks confirmation: "I heard X. I'll remember that means Y."
- Useful for training proper nouns and unusual words

---

### `ListPatterns() → Vec<(heard: String, meant: String, confidence: String)>`

List all learned voice patterns.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListPatterns
```

**Returns:** Array of (heard, meant, confidence_info) tuples.  
Example: `("ever", "abba", "80% (manual)")`

---

### `GetFingerprintStats() → (manual_count: u32, passive_count: u32, command_count: u32)`

Get statistics about the voice learning fingerprint.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetFingerprintStats
```

**Returns:**
- `manual_count`: Patterns from manual training
- `passive_count`: Patterns from passive LLM learning
- `command_count`: Total commands in history

---

## Pattern Import/Export (Phase 10)

### `ExportFingerprint(path: String) → bool`

Export learned patterns to a JSON file.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ExportFingerprint s "/home/user/Documents/voice_backup.json"
```

**Parameters:**
- `path`: Absolute path to export file (must be writable by service)

**Returns:** `true` if successful.

**Writable Paths:**
- `~/.local/share/speechd-ng/`
- `~/Documents/`

---

### `ImportFingerprint(path: String, merge: bool) → u32`

Import patterns from a JSON file.

```bash
# Merge (keeps existing, adds new)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ImportFingerprint sb "/path/to/patterns.json" true

# Replace (overwrites everything)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ImportFingerprint sb "/path/to/patterns.json" false
```

**Parameters:**
- `path`: Absolute path to import file
- `merge`: If `true`, adds new patterns without overwriting existing

**Returns:** Total pattern count after import.

---

### `GetFingerprintPath() → String`

Get the path to the fingerprint data file.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetFingerprintPath
```

**Returns:** Path (e.g., `~/.local/share/speechd-ng/fingerprint.json`)

---

## Ignored Commands Tracking (Phase 11)

### `GetIgnoredCommands() → Vec<(heard: String, timestamp: String, context: String)>`

List all unrecognized/failed ASR attempts.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetIgnoredCommands
```

**Returns:** Array of (heard, timestamp, context) tuples.

**Notes:**
- Automatically populated when LLM returns confused/error responses
- Max 50 commands stored
- Duplicates are filtered

---

### `ClearIgnoredCommands() → u32`

Clear all ignored commands.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ClearIgnoredCommands
```

**Returns:** Count of commands cleared.

---

### `CorrectIgnoredCommand(heard: String, meant: String) → bool`

Correct an ignored command and add it as a pattern.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service CorrectIgnoredCommand ss "plae musik" "play music"
```

**Parameters:**
- `heard`: The ignored ASR transcription
- `meant`: What the user actually intended

**Returns:** `true` if the command was found and corrected.

**Notes:**
- Removes from ignored list
- Adds as manual correction pattern (70% confidence)

---

### `AddIgnoredCommand(heard: String, context: String)`

Manually add a command to the ignored list (for testing/debugging).

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service AddIgnoredCommand ss "strt musik" "testing"
```

---

## Configuration (Phase 13)

### `GetSttBackend() → String`

Get the currently configured STT backend.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetSttBackend
```

**Returns:** `"vosk"` or `"wyoming"`.

---

### `GetStatus() → (bool, f32, String, u32)`

Returns a diagnostic summary:
1. `ai_enabled` (bool): Is the LLM active?
2. `passive_threshold` (f32): Confidence threshold for passive learning.
3. `stt_backend` (String): Current STT backend.
4. `patterns_count` (u32): Total number of learned voice patterns.

---

### `GetWyomingInfo() → (host: String, port: u16, model: String, auto_start: bool)`

Get configuration details for the Wyoming protocol integration.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetWyomingInfo
```

**Returns:**
- `host`: Wyoming server host (e.g., `127.0.0.1`)
- `port`: Wyoming server port (e.g., `10301`)
- `model`: Configured Whisper model (e.g., `tiny`, `base`)
- `auto_start`: Whether the server is auto-started

---

## Python Integration Example

```python
#!/usr/bin/env python3
"""
SpeechD-NG Python Client Example
"""
import dbus

class SpeechClient:
    def __init__(self):
        self.bus = dbus.SessionBus()
        self.service = self.bus.get_object('org.speech.Service', '/org/speech/Service')
        self.iface = dbus.Interface(self.service, 'org.speech.Service')
    
    def speak(self, text, voice=None):
        """Speak text with optional voice selection."""
        if voice:
            self.iface.SpeakVoice(text, voice)
        else:
            self.iface.Speak(text)
    
    def think(self, query):
        """Ask the AI about recent speech context."""
        return str(self.iface.Think(query))
    
    def add_correction(self, heard, meant):
        """Add a voice correction pattern."""
        return bool(self.iface.AddCorrection(heard, meant))
    
    def list_patterns(self):
        """List all learned patterns."""
        return [(str(h), str(m), str(c)) for h, m, c in self.iface.ListPatterns()]
    
    def get_ignored_commands(self):
        """Get all unrecognized commands."""
        return [(str(h), str(t), str(c)) for h, t, c in self.iface.GetIgnoredCommands()]
    
    def correct_ignored(self, heard, meant):
        """Correct an ignored command."""
        return bool(self.iface.CorrectIgnoredCommand(heard, meant))
    
    def get_stats(self):
        """Get fingerprint statistics."""
        manual, passive, commands = self.iface.GetFingerprintStats()
        return {'manual': int(manual), 'passive': int(passive), 'commands': int(commands)}
    
    def export_patterns(self, path):
        """Export patterns to file."""
        return bool(self.iface.ExportFingerprint(path))
    
    def import_patterns(self, path, merge=True):
        """Import patterns from file."""
        return int(self.iface.ImportFingerprint(path, merge))


# Usage
if __name__ == "__main__":
    client = SpeechClient()
    
    # Speak
    client.speak("Hello from Python!")
    
    # Add a correction
    client.add_correction("mozurt", "mozart")
    
    # Check stats
    stats = client.get_stats()
    print(f"Patterns: {stats['manual']} manual, {stats['passive']} passive")
    
    # Export
    client.export_patterns("/home/user/Documents/my_patterns.json")
```

---

## Rust Integration Example

```rust
use zbus::blocking::Connection;
use zbus::dbus_proxy;

#[dbus_proxy(
    interface = "org.speech.Service",
    default_service = "org.speech.Service",
    default_path = "/org/speech/Service"
)]
trait SpeechService {
    fn speak(&self, text: &str);
    fn add_correction(&self, heard: &str, meant: &str) -> bool;
    fn list_patterns(&self) -> Vec<(String, String, String)>;
    fn get_fingerprint_stats(&self) -> (u32, u32, u32);
}

fn main() -> zbus::Result<()> {
    let conn = Connection::session()?;
    let proxy = SpeechServiceProxyBlocking::new(&conn)?;
    
    proxy.speak("Hello from Rust!")?;
    proxy.add_correction("mozurt", "mozart")?;
    
    let (manual, passive, commands) = proxy.get_fingerprint_stats()?;
    println!("Stats: {} manual, {} passive, {} commands", manual, passive, commands);
    
    Ok(())
}
```

---

## Error Handling

Most methods return meaningful error values:
- `bool` methods return `false` on failure
- `String` methods return error messages like `"Error: <details>"`
- Empty arrays indicate no data

The service logs detailed errors to journald:
```bash
journalctl --user -u speechd-ng -f
```

---

## Security Notes

1. **Polkit Authorization**: Methods like `DownloadVoice`, `TrainWord`, and `Think` check Polkit permissions.
2. **Systemd Sandboxing**: The service runs with strict filesystem restrictions.
3. **Export Paths**: Only certain directories are writable (see `ExportFingerprint`).

---

*Last Updated: 2025-12-28*
