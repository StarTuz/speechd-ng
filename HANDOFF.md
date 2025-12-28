# Project Handoff: SpeechD-NG

## Current Context
We are building **SpeechD-NG**, a modern replacement for Linux command-line/desktop speech services. The project is written in **Rust** to ensure memory safety, speed, and concurrency.

## Status: Phase 6 In Progress (Input & Accessibility)
We have implemented Microphone access and STT wiring.

### 1. Functional Features
-   **Audio Pipeline**: Threaded audio engine + `cpal` Microphone stream.
-   **D-Bus API**: `Listen()` method records 3 seconds of audio and attempts transcription.
-   **STT**: Wires up to `whisper` CLI (must be installed).

### 2. Architecture & Security
-   **Audio**: `Ear` actor manages input stream. Offloaded to blocking thread.
-   **Security**: `Listen` gated by `org.speech.service.listen`.

## File Structure
```
src/
├── main.rs          # D-Bus (Listen added)
├── engine.rs        # Audio Output
├── ear.rs           # Audio Input (Recording + Transcribe)
├── backends/        # TTS Plugins
```

## Immediate Next Steps (Phase 6 Completion)
1.  **Orca Shim**: Mimicking `speech-dispatcher`.
2.  **Continuous Listening**: Wake word detection?

## Known Limitations
-   **STT**: Requires external `whisper` binary.

## Repository
-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
