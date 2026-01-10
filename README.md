# SpeechD-NG: The Next-Generation Linux Speech Daemon

![Version](https://img.shields.io/badge/version-1.0.0-blue.svg) ![Status](https://img.shields.io/badge/status-stable-green.svg)

**SpeechD-NG** is a modern, secure, and intelligent speech service designed for the Linux ecosystem. It replaces the aging `speech-dispatcher` with a window-manager agnostic, high-performance, and AI-ready architecture.

## 🚀 Mission

1. **Window Manager Agnostic**: Works flawlessly on GNOME, KDE, Sway, Hyprland, and raw TTYs.
2. **Service-Based**: Runs as a standard `systemd` user service.
3. **Secure by Design**: Uses D-Bus for IPC with strict isolation and Polkit authorization.
4. **AI-Ready**: Built to integrate with local LLMs (like Ollama) for contextual understanding.
5. **Neural First**: First-class support for high-quality Piper neural voices.
6. **Pure Rust Autonomous Mode**: Integrated native wake word detection (Wendy) for hands-free interaction.
7. **Self-Improving**: Passive and manual voice learning to correct transcription errors over time.
8. **Multimodal**: Can see and describe the screen via local computer vision (The Eye).

## 🏗 Architecture

| Component | Description |
|-----------|-------------|
| **The Daemon** | Rust + `zbus`. Lightweight D-Bus router. |
| **Audio Engine** | Multi-backend mixer (eSpeak-ng + Piper). |
| **The Ear** | Native audio capture with offline STT (Vosk/Whisper). Zero Python. |
| **The Eye** | Local Vision Model (Moondream 2) for screen analysis. |
| **The Cortex** | Async Ollama connector with token-based streaming. |
| **The Chronicler** | Local vector database and embedding engine for long-term memory. |
| **The Fingerprint** | Voice learning engine for STT error correction. |

## 🛠 Building & Installation

### Prerequisites

| Package | Purpose |
|---------|---------|
| Rust (Stable) | Build the daemon |
| `espeak-ng` | Fast synthesis fallback |
| `piper-tts` | High-quality neural synthesis |
| `libvosk` | Native headers for STT and Wake Word |
| `Ollama` | AI brain (optional) |

### Build

```bash
cargo build --release
```

### Installation

```bash
# Quick Install (Recommended)
./install.sh

# Manual Installation
# Copy binaries
cp target/release/speechd-ng ~/.local/bin/
cp target/release/speechd-control ~/.local/bin/

# Install systemd service
cp systemd/speechd-ng.service ~/.config/systemd/user/
systemctl --user daemon-reload

# Create required directories (Critical for sandboxing)
mkdir -p ~/.local/share/piper/models
mkdir -p ~/.local/share/speechd-ng
mkdir -p ~/Documents

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

# Wake Word (Hands-Free Mode)
wake_word = "wendy"                 # Phonetically distinct generic default
enable_wake_word = false

# Memory & OOM Protection
max_audio_size_mb = 50              # Protection against malicious Large-Payload ASR
playback_timeout_secs = 30
playback_volume = 1.0

# Phase 5: Chronicler (Long-term Memory)
enable_rag = true                   # Set to true to enable local memory
rag_top_k = 3                       # Number of relevant memories to retrieve
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
| **Brain Mgmt** | `GetBrainStatus`, `ManageBrain` |
| **Training** | `AddCorrection`, `TrainWord`, `ListPatterns`, `GetFingerprintStats` |
| **Import/Export** | `ExportFingerprint`, `ImportFingerprint`, `GetFingerprintPath` |
| **Ignored** | `GetIgnoredCommands`, `CorrectIgnoredCommand`, `ClearIgnoredCommands` |

## 🎤 Wake Word Mode

When enabled, the daemon listens for a wake word and responds:

1. Say "**Wendy**" (or your configured wake word)
2. Daemon responds: "Yes?"
3. Speak your command (4 second window)
4. AI processes and responds via TTS

Enable in config:

```toml
wake_word = "wendy"
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

## 👁️ Vision (The Eye)

The assistant can "see" your screen using a local Vision-Language Model (Moondream).

### CLI Usage

```bash
# Describe the current screen
speechd-control describe

# Ask specific questions
speechd-control describe "What is the error message in the terminal?"
```

### API Usage

```bash
busctl --user call org.speech.Service /org/speech/Service org.speech.Service DescribeScreen s "Describe this screen"
```

## 🎤 Piper TTS Setup

### Binary Conflict Warning

On some distributions (like Arch Linux/Garuda), the command `piper` refers to a gaming mouse configuration tool (`libratbag`). To use neural TTS, ensure you have the **neural engine** installed (often named `piper-tts` or found in AUR as `piper-tts-git`).

Update your `Speech.toml` to point to the correct binary:

```toml
piper_binary = "piper-tts"  # Or full path: /usr/bin/piper-tts
```

## 👥 Team & Governance

SpeechD-NG is guided by **The Council**, a committee of domain expert personas:

- **AI & Models**: Dr. Aris Thorne (Inference efficiency & quantization)
- **Systems & Latency**: Nikolai "Sprint" Volkov (Low-level performance & async Rust)
- **Blue Team (Defense)**: Sloane "Bulwark" Vance (Hardening & security integration)
- **Red Team (Offense)**: Kaelen "Viper" Cross (Adversarial testing & edge cases)
- **UX & Accessibility**: Elara Vance (Human factors & VUI design)

For more details on our governance model and expert mandates, see **[TEAM_EXPERTS.md](TEAM_EXPERTS.md)**.
All contributions must adhere to our **[GUARDRAILS.md](GUARDRAILS.md)**.

## 🔒 Security

- **Systemd Sandboxing**: 20+ security directives. Note that `ProtectHome` and `PrivateDevices` are relaxed to allow screen capture and audio hardware access.
- **Polkit Integration**: Permission checks on sensitive operations (`Think`, `Listen`, `DescribeScreen`).
- **Isolation**: The daemon runs as a unprivileged user service.

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
