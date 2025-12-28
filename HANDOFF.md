# Project Handoff: SpeechD-NG

## Current Context
We are building **SpeechD-NG**, a modern replacement for Linux command-line/desktop speech services. The project is written in **Rust** to ensure memory safety, speed, and concurrency.

## Status: Phase 4 Completed (Security & Configuration)
We have implemented all core functionality through Phase 4.

### 1. Functional Features
-   **D-Bus Service**: Claims `org.speech.Service` on the user session bus.
-   **Async Core**: Uses `tokio` and `zbus` for non-blocking I/O.
-   **Audio Pipeline**: Dedicated thread handles `espeak-ng` + `rodio` playback.
-   **The Cortex**: Async module for speech memory and Ollama integration.
-   **Security Hooks**: `SecurityAgent` validates sender on sensitive methods.
-   **Configuration**: Dynamic settings via `Speech.toml`, environment vars, or defaults.

### 2. Key Technical Decisions
-   **Rodio v0.17.3**: Pinned for API stability.
-   **Dual-Actor Architecture**: 
    -   `AudioEngine` (Sync Thread): Blocking audio operations.
    -   `Cortex` (Async Task): HTTP/Ollama and state management.
    -   `SpeechService` (Main): D-Bus dispatch to both actors.
-   **Systemd Sandboxing**: 20+ security directives applied.

## File Structure
```
src/
├── main.rs          # Entry point, D-Bus interface
├── engine.rs        # Audio synthesis actor
├── cortex.rs        # Intelligence & Memory actor
├── security.rs      # Permission validation (Polkit stub)
└── config_loader.rs # Dynamic configuration
systemd/
├── speechd-ng.service         # Hardened systemd unit
└── org.speech.Service.service # D-Bus activation file
```

## Immediate Next Steps (Phase 5)
The next milestone is **Plug-in & Voice System**.

1.  **Backend Trait**: Abstract the TTS engine so `espeak-ng`, `Piper`, `Coqui`, etc. can be swapped.
2.  **Voice Enumeration**: D-Bus method to list available voices.
3.  **Voice Selection**: Allow callers to specify voice in `Speak()`.

## Known Limitations
-   **Polkit**: Security hook logs but does not yet enforce denial (stub implementation).
-   **LLM Sanitization**: User prompts are not sanitized before sending to Ollama.

## Repository
-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
