# SpeechD-NG D-Bus API Reference

This document provides a complete reference for GUI developers and integrators who want to interact with the SpeechD-NG daemon via D-Bus.

> **Note:** For command-line usage, see the [CLI Manual](CLI_MANUAL.md).

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

### `GetVersion() → String` (Phase 14)

Get the daemon version string.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetVersion
```

**Returns:** Version string (e.g., `"0.2.0"`)

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

### `CheckWyomingHealth() → (is_reachable: bool, message: String)`

Check if the Wyoming server is reachable.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service CheckWyomingHealth
```

**Returns:**

- `is_reachable`: `true` if the server is reachable.
- `message`: Status message (e.g., "Successfully connected to Wyoming at 127.0.0.1:10301")

---

## Streaming Media Player (Phase 15)

### `PlayAudio(url: String) → String`

Play audio from a URL. Downloads the audio file and plays it through the audio engine.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service PlayAudio s "https://example.com/audio.wav"
```

**Parameters:**

- `url`: HTTP/HTTPS URL to audio file (WAV, MP3, OGG, FLAC supported)

**Returns:** Empty string on success, error message on failure.

**Configuration** (`~/.config/speechd-ng/Speech.toml`):

```toml
max_audio_size_mb = 50       # Max file size in MB
playback_timeout_secs = 30   # Download timeout
playback_volume = 1.0        # Default volume (0.0-1.0)
```

**Notes:**

- Audio is fully downloaded before playback (no streaming)
- Queues behind TTS (does not interrupt)
- URL must start with `http://` or `https://`

---

### `StopAudio() → bool`

Stop current audio playback.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service StopAudio
```

**Returns:** `true` if playback was stopped, `false` if nothing was playing.

---

### `SetVolume(volume: f64) → bool`

Set the playback volume (affects both TTS and media playback).

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service SetVolume d 0.7
```

**Parameters:**

- `volume`: Volume level from 0.0 (mute) to 1.0 (full)

**Returns:** `true` on success.

---

### `GetVolume() → f64`

Get the current playback volume setting.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetVolume
```

**Returns:** Current volume (0.0 - 1.0).

---

### `GetPlaybackStatus() → (is_playing: bool, current_url: String)`

Get the current playback status.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetPlaybackStatus
```

**Returns:**

- `is_playing`: Whether audio is currently playing
- `current_url`: URL of currently playing audio (empty if not playing)

---

## Rate Limiting (Phase 17b)

To prevent abuse, the daemon enforces rate limits per D-Bus sender.

| Method Type | Default Limit | Protected Methods |
|-------------|---------------|-------------------|
| **TTS** | 30/min | `Speak`, `SpeakVoice`, `SpeakChannel` |
| **AI** | 10/min | `Think` |
| **Audio** | 20/min | `PlayAudio`, `PlayAudioChannel` |
| **Listen** | 30/min | `Listen`, `ListenVad` |

**Behavior:**

- If the limit is exceeded, the method returns a **D-Bus Error**: `org.freedesktop.DBus.Error.Failed: Rate limited`.
- A log entry is created: `Rate limited: <Type> for sender :1.xxxx`.

---

## Multi-Channel Audio (Phase 16a/16c)

### `SpeakChannel(text: String, voice: String, channel: String) → bool`

Speak text to a specific audio channel. Supports Stereo (2ch) and 5.1 Surround (6ch).

```bash
# Speak to left ear only (Stereo)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service SpeakChannel sss "Tower, left ear" "" "left"

# Speak to Rear Left (5.1 Surround)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service SpeakChannel sss "Traffic behind you" "" "rear-left"
```

**Parameters:**

- `text`: Text to speak
- `voice`: Voice ID (empty for default)
- `channel`: Target channel identifier (case-insensitive)

| Channel Key | Description | Output Mode |
|-------------|-------------|-------------|
| `left`, `front-left` | Left Channel | Stereo (2ch) |
| `right`, `front-right` | Right Channel | Stereo (2ch) |
| `center` | Phantom Center (70% L/R) | Stereo (2ch) |
| `stereo` | Full Stereo | Stereo (2ch) |
| `rear-left` | Surround Left | Surround (5.1) |
| `rear-right` | Surround Right | Surround (5.1) |
| `center-real` | Discrete Center | Surround (5.1) |
| `lfe`, `subwoofer` | LFE / Sub | Surround (5.1) |

**Returns:** `true` on success.

---

### `PlayAudioChannel(url: String, channel: String) → String`

Play audio from URL to a specific channel. Supports same channel keys as `SpeakChannel`.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service PlayAudioChannel ss "https://example.com/siren.wav" "lfe"
```

**Returns:** Empty string on success, error message on failure.

---

### `ListChannels() → Vec<(name: String, description: String)>`

List available audio channels configurations.

---

## PipeWire Device Routing (Phase 16b)

### `ListSinks() → Vec<(id: u32, name: String, desc: String, is_default: bool)>`

List available PipeWire audio output devices.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListSinks
# Returns: a(ussb) 1 68 "SB Omni Surround 5.1" "SB Omni Surround 5.1" true
```

---

### `GetDefaultSink() → (id: u32, name: String)`

Get the current default audio sink.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetDefaultSink
# Returns: us 68 "SB Omni Surround 5.1 Analog Surround 5.1"
```

---

### `SpeakToDevice(text: String, voice: String, device_id: u32) → bool`

Speak text to a specific PipeWire device by ID. Temporarily sets the device as default, speaks, then restores.

```bash
# Get available sinks first
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListSinks

# Speak to device 68 (SB Omni)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service SpeakToDevice ssu "Hello headset" "" 50

# Speak to device 50 (G533 Headset)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service SpeakToDevice ssu "Hello speakers" "" 68
```

**Returns:** `true` on success.

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

---

## Local AI (Ollama) Management (Phase 19)

These methods allow clients to monitor and control the local reasoning engine (Ollama).

### `GetBrainStatus() → (is_running: Bool, current_model: String, available_models: Array[String])`

Polls the Ollama API to verify health and list downloaded models.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetBrainStatus
```

**Returns:**

- `is_running`: `true` if Ollama API is reachable.
- `current_model`: The model currently configured in `Speech.toml`.
- `available_models`: List of models already downloaded and ready to use.

---

### `ManageBrain(action: String, param: String) → Bool`

Perform administrative actions on the Ollama service.

```bash
# Start the service
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ManageBrain ss "start" ""

# Pull a new model
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ManageBrain ss "pull" "llama3"

# Switch to a model (alias for SetBrainModel)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ManageBrain ss "use" "llama3:latest"
```

**Parameters:**

- `action`:
  - `"start"`: Attempts to start the `ollama` service via `systemctl`.
  - `"stop"`: Attempts to stop the `ollama` service via `systemctl`.
  - `"pull"`: Initiates a model download.
  - `"use"`: Switch to a different model at runtime (same as `SetBrainModel`).
- `param`: Model name for `"pull"` or `"use"` actions (e.g., `"llama3:latest"`).

**Returns:** `true` if the operation was initiated successfully.

---

### `SetBrainModel(model: String) → Bool`

Switch the AI model at runtime without restarting the daemon.

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service SetBrainModel s "llama3:latest"
```

**Parameters:**

- `model`: Model name to switch to (e.g., `"llama3:latest"`, `"mistral:latest"`).

**Returns:** `true` if the model was switched successfully.

**Notes:**

- Does not validate model availability (trusts user input).
- Change persists until daemon restart; to persist permanently, update `Speech.toml`.
- Check available models with `GetBrainStatus()`.

---

## Error Handling

- **Rate Limits**: Return `org.freedesktop.DBus.Error.Failed` (Message: "Rate limited").
- **Permissions**: Return `org.freedesktop.DBus.Error.AccessDenied` (Message: "Polkit denied").
- **Logic Errors**: `bool` methods return `false`, `String` methods return `"Error: <details>"`.
- **Empty Data**: Empty arrays indicate no data found.

The service logs detailed errors to journald for debugging:

```bash
journalctl --user -u speechd-ng -f
```

---

## Security Notes

1. **Polkit Authorization**: Sensitive methods check permissions via `zbus_polkit`.
   - `DownloadVoice`, `TrainWord`, `Think`, `Listen`: Require `org.speech.service.*` privileges.
   - **Active Desktop**: Typically auto-allowed.
   - **Remote/SSH**: Requires admin authentication.
2. **Rate Limiting**: Enforced per sender unique name (e.g., `:1.55`) to prevent DoS.
3. **Systemd Sandboxing**: The service runs with strict filesystem restrictions (`ProtectSystem=strict`, `ProtectHome=read-only`).
4. **Export Paths**: Only specific directories are writable (e.g., `~/.local/share/speechd-ng/`).

---

*Last Updated: 2025-12-30 (v0.7.2)*
