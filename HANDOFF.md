# Project Handoff: SpeechD-NG

## Current Context
We are building **SpeechD-NG**, a modern replacement for Linux command-line/desktop speech services. The project is written in **Rust** to ensure memory safety, speed, and concurrency.

## Status: Phase 6 Completed (Input & Accessibility)
We have implemented Microphone access, STT wiring, and Legacy Compatibility.

### 1. Functional Features
-   **Audio Pipeline**: Threaded audio engine + `cpal` Microphone stream.
-   **D-Bus API**: `Listen()` method records 3 seconds of audio and attempts transcription.
-   **SSIP Shim**: Listens on TCP 6560 to accept legacy `speech-dispatcher` commands (e.g. from Orca).
-   **STT**: Wires up to `whisper` CLI.

### 2. Architecture & Security
-   **Audio Input**: `Ear` actor manages input stream. Offloaded to blocking thread.
-   **Legacy Compat**: `ssip.rs` implements partial Speech Synthesis Interface Protocol.
-   **Security**: `Listen` gated by Polkit. SSIP via local TCP (Implicit trust).

## File Structure
```
src/
├── main.rs          # D-Bus & SSIP Task Launcher
├── engine.rs        # Audio Output
├── ear.rs           # Audio Input (Recording + Transcribe)
├── ssip.rs          # Legacy Protocol Shim (TCP 6560)
├── backends/        # TTS Plugins
```

## Immediate Next Steps (Future)
-   **Continuous Listening**: Wake word detection.
-   **Refinement**: Improve SSIP coverage (Events, Indices).
-   **Packaging**: Create .deb/rpm/flatpak.

## Known Limitations
-   **STT**: Requires external `whisper` binary.
-   **SSIP**: Minimal implementation (SPEAK/SET only).

## Repository
-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
