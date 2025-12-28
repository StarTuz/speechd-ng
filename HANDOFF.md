# Project Handoff: SpeechD-NG

## Current Context
We are building **SpeechD-NG**, a modern replacement for Linux command-line/desktop speech services. The project is written in **Rust** to ensure memory safety, speed, and concurrency.

## Status: Phase 5 Partial (Plug-ins & Security)
We have established the **Plugin Architecture** and reinforced security.

### 1. Functional Features
-   **D-Bus Service**: Claims `org.speech.Service`.
-   **Audio Pipeline**: Threaded audio engine with **Process Timeouts**.
-   **The Cortex**: Aware of speech history and connected to Ollama.
-   **Security**:
    -   **Systemd Sandbox**: Strict file/network/capability restrictions.
    -   **LLM Sanitization**: Hardened prompts.
    -   **Backend Safety**: 5-second timeout on speech synthesis processes.

### 2. Architecture: Pluggable Backends
-   **`SpeechBackend` Trait**: Located in `src/backends/mod.rs`. Allows easy addition of new TTS engines.
-   **Current Backends**:
    -   `EspeakBackend`: Wraps `espeak-ng` binary. Includes timeout logic using `wait-timeout` crate.
-   **Isolation**: Backends run in the Audio Thread, decoupled from the main D-Bus loop.

## File Structure
```
src/
├── main.rs          # D-Bus & Cortex/Engine orchestration
├── engine.rs        # Audio Thread (Consumer of Backends)
├── cortex.rs        # Memory & AI
├── security.rs      # Polkit Stubs
├── config_loader.rs # Settings
└── backends/        # TTS Plugins
    ├── mod.rs       # Trait definition
    └── espeak.rs    # Espeak implementation
```

## Immediate Next Steps (Phase 5 Completion)
1.  **Voice Enumeration**: Add `ListVoices()` to the D-Bus API.
2.  **Voice Selection**: Update `Speak()` to accept a voice ID.
3.  **Piper Backend**: Implement `SpeechBackend` for the high-quality Piper TTS.

## Known Limitations
-   **Polkit**: Not yet blocking unprivileged calls (Hook exists).
-   **Voice Config**: Hardcoded to default voice in backend.

## Repository
-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
