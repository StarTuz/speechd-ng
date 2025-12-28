# Development Roadmap

This document outlines the strategic phases for SpeechD-NG development.

## Phase 1: Foundation (✅ Completed)
**Goal**: Establish the D-Bus presence and process architecture.
-   [x] Initialize Rust project.
-   [x] Implement `zbus` for `org.speech.Service`.
-   [x] Create Systemd service files.
-   [x] Verify IPC via `busctl`.

## Phase 2: Audio Engine (✅ Completed)
**Goal**: Reliable, non-blocking text-to-speech.
-   [x] Integrate `espeak-ng` via CLI/Bindings.
-   [x] Integrate `rodio` for audio playback.
-   [x] Implement Threaded Actor model (isolating audio from main loop).
-   [x] Handle "firehose" requests (queueing).

## Phase 3: The Cortex ("Brain & Body") (✅ Completed)
**Goal**: Centralize Intelligence in the Daemon, expose it via API.
-   [x] **Cortex Module**: Async actor holding the "Short-Term Memory" (Speech History).
-   [x] **Omniscient API**: D-Bus methods for clients to query context (e.g., `QueryContext`, `SummarizeHistory`).
-   [x] **Ollama Connector**: HTTP client to talk to `localhost:11434` for processing queries.
-   [ ] **Reference Client**: A simple CLI tool (simulating a WM widget) to demonstrate "Asking the Daemon" about what was said.

## Phase 4: Plug-in & Voice System
**Goal**: Extensibility.
-   [ ] **Backend Trait**: Abstract `espeak-ng` so we can plugin `Piper`, `Coqui`, etc.
-   [ ] **Voice Enumeration**: D-Bus API to list available voices.
-   [ ] **Configuration**: `config.toml` for standard settings.

## Phase 5: Security & Permissions
**Goal**: Hardening for multi-user/untrusted environments.
-   [ ] **Polkit**: Require authorization for "listening" or "configuring".
-   [ ] **Socket Activation**: Ensure daemon only runs when needed (refine current setup).
-   [ ] **Namespace Isolation**: Use systemd sandboxing (`ProtectHome`, `PrivateNetwork` except localhost).

## Phase 6: Input & Accessibility
**Goal**: Two-way interaction.
-   [ ] **Microphone Stream**: Secure access to mic.
-   [ ] **Speech-to-Text (STT)**: Integration with Whisper (cpp or burn).
-   [ ] **Orca Compatibility**: Shim to pretend to be `speech-dispatcher` for legacy app support.
