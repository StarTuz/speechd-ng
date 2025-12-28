# Project Handoff: SpeechD-NG

## Current Context
We are building **SpeechD-NG**, a modern replacement for Linux command-line/desktop speech services. The project is written in **Rust** to ensure memory safety, speed, and concurrency.

## Status: Phase 5 Completed (Plug-ins & Voice System)
We have implemented a fully extensible Voice System with Security Hardening.

### 1. Functional Features
-   **Audio Pipeline**: 
    -   Threaded audio engine with **Process Timeouts** (5s).
    -   Parses `espeak-ng` voices dynamically.
-   **D-Bus API**: 
    -   `ListVoices()`: Returns `(VoiceID, Name)` pairs.
    -   `SpeakVoice(text, voice_id)`: Speaks using the selected voice.
-   **Architecture**:
    -   **Pluggable Backends**: Traits allow swapping TTS engines.
    -   **EspeakBackend**: Fully implemented.

### 2. Architecture & Security
-   **Timeouts**: Synthesizer processes are killed if they hang, preventing DoS.
-   **Sandboxing**: Systemd strict confinement.
-   **LLM Safety**: Prompt injection filtering.

## File Structure
```
src/
├── main.rs          # D-Bus (Speak, SpeakVoice, ListVoices, Think)
├── engine.rs        # Audio Actor (Voice Selection logic)
├── backends/        # TTS Plugins
│   ├── mod.rs       # Trait definition + Voice struct
│   └── espeak.rs    # Espeak implementation (parsing & synthesis)
```

## Immediate Next Steps (Phase 6)
The next milestone is **Input & Accessibility**.

1.  **Microphone Access**: Integrating a secure audio recorder stream.
2.  **Speech-to-Text**: Using Whisper (via `burn` or `whisper.cpp`) to transact audio->text.
3.  **Orca Shim**: Mimicking `speech-dispatcher` to support legitimate screen reader clients.

## Known Limitations
-   **Voice Config Persistence**: The daemon doesn't remember the "default" voice across restarts (must be passed in `SpeakVoice` or defaults to espeak default).

## Repository
-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
