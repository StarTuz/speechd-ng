# Project Handoff: SpeechD-NG

## Current Context
We have completed **Phase 7** of the roadmap. **SpeechD-NG** is now a fully capable, secure, and hands-free speech assistant for Linux. It supports high-quality neural voices and autonomous voice interaction.

## Status: Phase 7 Completed (Hands-Free & Neural Voices)

### 1. Functional Features
-   **Neural TTS (Piper)**: Seamless integration with Piper. Intelligent mixer handles `piper:` and `espeak:` prefixes.
-   **Zero-Config Downloader**: Securely download high-fidelity voices from Hugging Face via D-Bus.
-   **Autonomous Mode**: Background wake word listener ("StarTuz") triggers command capture.
-   **Voice Command Loop**: STANDBY -> WAKE -> CAPTURE -> THINK -> RESPOND.
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
├── backends/        # TTS Engines (Piper, eSpeak)
├── ssip.rs          # Legacy Shim
└── wakeword_bridge.py # Python/Vosk Standby Listener
```

## Setup & Testing
1. Ensure `vosk-transcriber` and `piper` are in PATH.
2. Set `enable_wake_word = true` in `~/.config/speechd-ng/Speech.toml`.
3. Say **"StarTuz"** to trigger.

## Known Limitations
-   **Microphone Exclusivity**: If another app uses the mic exclusively, the wake word listener might fail (PulseAudio/Pipewire handles this usually).
-   **Manual Paths**: The bridge looks for models in `~/.cache/vosk/`.

## Repository
-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
