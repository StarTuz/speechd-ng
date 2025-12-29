# SpeechD-NG: The Next-Generation Linux Speech Daemon

**SpeechD-NG** is a modern, secure, and intelligent speech service designed for the Linux ecosystem. It replaces the aging `speech-dispatcher` with a window-manager agnostic, high-performance, and AI-ready architecture.

## 🚀 Mission

1.  **Window Manager Agnostic**: Works flawlessly on GNOME, KDE, Sway, Hyprland, and raw TTYs.
2.  **Service-Based**: Runs as a standard `systemd` user service.
3.  **Secure by Design**: Uses D-Bus for IPC with strict isolation and Polkit authorization.
4.  **AI-Ready**: Built to integrate with local LLMs (like Ollama) for contextual understanding.
5.  **Neural First**: First-class support for high-quality Piper neural voices.
6.  **Autonomous**: Integrated wake word detection for hands-free interaction.
7.  **Self-Improving**: Passive and manual voice learning to correct transcription errors over time.

## 🏗 Architecture

| Component | Description |
|-----------|-------------|
| **The Daemon** | Rust + `zbus`. Lightweight D-Bus router. |
| **Audio Engine** | Multi-backend mixer (eSpeak-ng + Piper). |
| **The Ear** | Audio capture with offline STT (Vosk/Whisper). |
| **The Cortex** | Async Ollama connector for AI "thinking". |
| **The Fingerprint** | Voice learning engine for STT error correction. |

## 🛠 Building & Installation

### Prerequisites

| Package | Purpose |
|---------|---------|
| Rust (Stable) | Build the daemon |
| `espeak-ng` | Fast synthesis fallback |
| `piper` | High-quality neural synthesis |
| `vosk` (pip) | Wake word and STT |
| `Ollama` | AI brain (optional) |

### Build

```bash
cargo build --release
```

### Installation

```bash
# Copy binary
cp target/release/speechserverdaemon ~/.local/bin/

# Copy Python bridges (Required for Wake Word & Wyoming)
cp src/wakeword_bridge.py ~/.local/bin/
cp src/wyoming_bridge.py ~/.local/bin/
chmod +x ~/.local/bin/*.py

# Install systemd service
cp systemd/speechd-ng.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now speechd-ng
```

### Configuration

Create `~/.config/speechd-ng/Speech.toml`:

```toml
# LLM Settings
ollama_url = "http://localhost:11434"
ollama_model = "llama3"
enable_ai = true                    # Set to false to disable LLM features (saves RAM)

# TTS Settings
piper_model = "en_US-lessac-medium"
piper_binary = "piper"              # Or full path: /path/to/piper
tts_backend = "piper"               # "piper" or "espeak"

# Speech-to-Text (Vosk or Wyoming)
stt_backend = "wyoming"             # "vosk" is default
wyoming_host = "127.0.0.1"
wyoming_port = 10301

# Memory & Audio
memory_size = 50
enable_audio = true

# Wake Word (Hands-Free Mode)
wake_word = "mango"
enable_wake_word = false
```


## 📡 Quick Start

```bash
# Speak something
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Speak s "Hello world"

# List available voices
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListVoices

# Ask the AI about recent speech
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Think s "Summarize what was said"

# Add a voice correction
busctl --user call org.speech.Service /org/speech/Service org.speech.Service AddCorrection ss "mozurt" "mozart"

# View all learned patterns
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListPatterns
```

## 📖 Full API Documentation

See **[docs/API_REFERENCE.md](docs/API_REFERENCE.md)** for the complete D-Bus API reference including:

- All methods with parameters and return types
- Python integration examples
- Rust integration examples
- Error handling guidelines

| Category | Methods |
|----------|---------|
| **Core** | `Ping`, `GetVersion`, `GetStatus` |
| **TTS** | `Speak`, `SpeakVoice`, `ListVoices`, `ListDownloadableVoices`, `DownloadVoice` |
| **AI** | `Think`, `Listen` |
| **Training** | `AddCorrection`, `TrainWord`, `ListPatterns`, `GetFingerprintStats` |
| **Import/Export** | `ExportFingerprint`, `ImportFingerprint`, `GetFingerprintPath` |
| **Ignored** | `GetIgnoredCommands`, `CorrectIgnoredCommand`, `ClearIgnoredCommands` |

## 🎤 Wake Word Mode

When enabled, the daemon listens for a wake word and responds:

1. Say "**Mango**" (or your configured wake word)
2. Daemon responds: "Yes?"
3. Speak your command (4 second window)
4. AI processes and responds via TTS

Enable in config:
```toml
wake_word = "mango"
enable_wake_word = true
```

## 🧠 Voice Learning

SpeechD-NG learns from your voice to improve accuracy:

### Passive Learning
When the LLM corrects an ASR error, the system automatically learns the pattern.

### Manual Training
Explicitly teach the system words it mishears:

```bash
# Direct correction (you know what ASR gets wrong)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service AddCorrection ss "mozurt" "mozart"

# Interactive training (record and learn)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service TrainWord su "beethoven" 3
```

### Pattern Management

```bash
# View all patterns
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListPatterns

# Export for backup/sharing
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ExportFingerprint s "$HOME/Documents/voice_patterns.json"

# Import from another system
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ImportFingerprint sb "/path/to/patterns.json" true
```

## 🗺 Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | Foundation (D-Bus + Systemd) | ✅ Complete |
| 2 | Audio Engine (rodio + eSpeak) | ✅ Complete |
| 3 | The Cortex (Ollama + History) | ✅ Complete |
| 4 | Security (Polkit + Sandbox) | ✅ Complete |
| 5 | Premium Voices (Piper) | ✅ Complete |
| 6 | Accessibility (STT + SSIP) | ✅ Complete |
| 7 | Autonomous (Wake Word) | ✅ Complete |
| 8 | Passive Learning | ✅ Complete |
| 9 | Manual Training API | ✅ Complete |
| 10 | Pattern Import/Export | ✅ Complete |
| 11 | Ignored Commands | ✅ Complete |
| 12 | Improved VAD | ✅ Complete |
| 14 | Hardening & Packaging | ✅ Complete |
| 15 | Streaming Media Player | ✅ Complete |
| 16a | Multi-Channel Audio | ✅ Complete |

## 🔒 Security

- **Systemd Sandboxing**: 20+ security directives
- **Polkit Integration**: Permission checks on sensitive operations
- **Read-Only Home**: Writes only to specific directories
- **No Network Abuse**: Restricted to localhost and Hugging Face

## 📝 License

MIT License - See [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions welcome! Please see:
- [ROADMAP.md](ROADMAP.md) - Development phases
- [examples/python_client.py](examples/python_client.py) - Ready-to-run Python client
- [docs/API_REFERENCE.md](docs/API_REFERENCE.md) - API documentation
- [HANDOFF.md](HANDOFF.md) - Current status and quick reference

---

*SpeechD-NG: Speak freely.*
