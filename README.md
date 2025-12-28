# SpeechD-NG: The Next-Generation Linux Speech Daemon

**SpeechD-NG** is a modern, secure, and intelligent speech service designed for the Linux ecosystem. It aims to replace the aging `speech-dispatcher` with a window-manager agnostic, high-performance, and "AI-ready" architecture.

## 🚀 Mission
1.  **Window Manager Agnostic**: Works flawlessly on GNOME, KDE, Sway, Hyprland, and raw TTYs.
2.  **Service-Based**: Runs as a standard `systemd` service (User or System).
3.  **Secure by Design**: Uses D-Bus for IPC with strict isolation and Polkit authorization.
4.  **AI-Ready**: Built to integrate with local LLMs (like Ollama) for passive/active learning.
5.  **Neural First**: First-class support for high-quality Piper neural voices with automated model downloading.
6.  **Autonomous**: Integrated wake word detection for hands-free interaction.
7.  **Self-Improving**: Passive and active voice learning to correct transcription errors over time.

## 🏗 Architecture

1.  **The Daemon (Core)**: Rust + `zbus`. Extremely lightweight router.
2.  **The Audio Engine**: Multi-backend mixer supporting `eSpeak-ng` and `Piper`.
3.  **The Ear**: Native audio capture with offline STT (Vosk/Whisper) and Wake Word detection.
4.  **The Cortex**: Async Ollama connector for context-aware "thinking" and summaries.
5.  **The Fingerprint**: Local learning engine that tracks voice patterns and corrects STT errors.

## 🛠 Building & Installation

### Prerequisites
-   Rust (Stable)
-   `espeak-ng` (Runtime for fast synthesis)
-   `piper` (High-quality neural synthesis)
-   `vosk` (Python package for wake word and STT)
-   `Ollama` (Optional, for "Brain" features)

### Build
```bash
cargo build --release
```

### Installation (User Service)
1.  Copy the binary:
    ```bash
    cp target/release/speechserverdaemon ~/.local/bin/
    ```
2.  Install Systemd Unit:
    ```bash
    cp systemd/speechd-ng.service ~/.config/systemd/user/
    systemctl --user daemon-reload
    systemctl --user enable --now speechd-ng
    ```

### Configuration
Create `~/.config/speechd-ng/Speech.toml` with any of these options:
```toml
ollama_url = "http://localhost:11434"   # LLM endpoint
ollama_model = "llama3"                 # LLM model name
piper_model = "en_US-lessac-medium"     # Default Piper voice
piper_binary = "piper"                  # Path to piper binary (or just name for PATH lookup)
tts_backend = "piper"                   # Default backend: "piper" or "espeak"
memory_size = 50                        # Context memory size
enable_audio = true                     # Audio output toggle
wake_word = "startuz"                   # Wake word phrase
enable_wake_word = false                # Enable hands-free mode
```

## 📡 API Usage (D-Bus)

### Example: Command Line
```bash
# Speak (Premium Neural Voice)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Speak s "Hello world"

# List All Remote Neural Voices
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListDownloadableVoices

# Download a Neural Voice
busctl --user call org.speech.Service /org/speech/Service org.speech.Service DownloadVoice s "piper:en_US-amy-low"

# Hands-Free Interaction
# Simply say "StarTuz" (or your configured wake word)
# The daemon will respond "Yes?" and record your next 4 seconds of speech.
```

## 🗺 Roadmap

-   **Phase 1: Foundation** (✅ Core D-Bus)
-   **Phase 2: Audio Engine** (✅ rodio + eSpeak)
-   **Phase 3: The Cortex** (✅ Ollama + History)
-   **Phase 4: Security** (✅ Polkit + Systemd Sandboxing)
-   **Phase 5: Premium Voices** (✅ Piper + Zero-Config Downloader)
-   **Phase 6: Accessibility** (✅ STT + SSIP/Orca Shim)
-   **Phase 7: Autonomous** (✅ Wake Word + Command Loop)
-   **Phase 8: Voice Learning** (✅ Personalized Fingerprinting)
