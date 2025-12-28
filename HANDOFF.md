# Project Handoff: SpeechD-NG

## Current Context
We have completed **Phase 8** of the roadmap. **SpeechD-NG** is now a self-improving, hands-free speech assistant for Linux.

## Status: Phase 8 Completed (Personalized Learning)

### 1. Functional Features
-   **Neural TTS (Piper)**: Seamless integration with Piper. Intelligent mixer handles `piper:` and `espeak:` prefixes.
-   **Zero-Config Downloader**: Securely download high-fidelity voices from Hugging Face via D-Bus.
-   **Autonomous Mode**: Background wake word listener ("StarTuz") triggers command capture.
-   **Voice Command Loop**: STANDBY -> WAKE -> CAPTURE -> THINK -> RESPOND.
-   **Passive Learning**: Automatically identifies and learns from STT errors corrected by `Cortex`.
-   **Contextual Hints**: Injects user-specific voice patterns into LLM prompts to improve accuracy.
-   **Legacy Compat**: SSIP Shim (TCP 6560) for Orca support.

### 2. Architecture & Security
-   **Hybrid Input**: Rust `Ear` uses a Python-Vosk bridge (`wakeword_bridge.py`) for efficient, stable keyword spotting.
-   **Managed Downloads**: Piper models are stored in `~/.local/share/piper/models`.
-   **Systemd Sandbox**: Updated to allow network access to Hugging Face and write access to model directories.
-   **Polkit**: enforced for `Listen`, `Think`, and `DownloadVoice` methods.

## File Structure
```
src/
├── main.rs          # D-Bus & Autonomous Loop
├── engine.rs        # Audio Engine (Mixer)
├── ear.rs           # Audio Input (Manual & Autonomous)
├── cortex.rs        # Memory & LLM Integration
├── fingerprint.rs   # Voice Learning & Pattern Storage
├── backends/        # TTS Engines (Piper, eSpeak)
├── ssip.rs          # Legacy Shim
└── wakeword_bridge.py # Python/Vosk Standby Listener
```

## Setup & Testing
1. Ensure `vosk-transcriber` is installed (`pip install vosk`).
2. Install Piper and set `piper_binary` in config (or ensure `piper` is in PATH).
3. Set `enable_wake_word = true` in `~/.config/speechd-ng/Speech.toml`.
4. Say **"StarTuz"** to trigger (configurable via `wake_word` setting).

## Configuration Options
All settings are in `~/.config/speechd-ng/Speech.toml`:
| Setting | Default | Description |
|---------|---------|-------------|
| `piper_binary` | `piper` | Path to Piper executable |
| `piper_model` | `en_US-lessac-medium` | Default neural voice |
| `tts_backend` | `espeak` | Default TTS: `piper` or `espeak` |
| `wake_word` | `startuz` | Trigger phrase for autonomous mode |
| `enable_wake_word` | `false` | Enable hands-free listening |

## Known Limitations
-   **Microphone Exclusivity**: If another app uses the mic exclusively, the wake word listener might fail (PulseAudio/Pipewire handles this usually).
-   **Vosk Model Path**: The bridge looks for models in `~/.cache/vosk/`.
-   **Piper Binary Conflict**: If `/usr/bin/piper` exists (a different GTK app), you must set `piper_binary` to the correct Piper TTS path in your config.

## Repository
-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
