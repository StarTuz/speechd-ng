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

## Phase 4: Security & Configuration (✅ Completed)
**Goal**: Hardening and User Control.
-   [x] **Polkit Integration**: Security hook implemented (logs sender, enforcement via policy pending).
-   [x] **Configuration**: Load `Speech.toml` for customizable settings (Ollama URL, Memory Size, Audio Toggle).
-   [x] **Systemd Sandboxing**: 20+ security directives applied to service file.
-   [x] **LLM Prompt Sanitization**: User input filtered to prevent injection attacks.

## Phase 5: Plug-in & Voice System (✅ Completed)
**Goal**: Extensibility.
-   [x] **Backend Trait**: Abstract `espeak-ng` so we can plugin `Piper`, `Coqui`, etc.
-   [x] **Safety Timeouts**: Backends are killed if they hang (>5s).
-   [x] **Voice Enumeration**: D-Bus API to list available voices (`ListVoices`).
-   [x] **Voice Selection**: `SpeakVoice()` method to speak with a specific voice ID.

## Phase 6: Input & Accessibility (✅ Completed)
**Goal**: Two-way interaction (The "Ears").
-   [x] **Microphone Stream**: Secure access to mic via `cpal`.
-   [x] **Speech-to-Text (STT)**: Integration with Whisper (CLI) and Vosk (CLI).
-   [x] **Orca Compatibility**: Shim to pretend to be `speech-dispatcher` for legacy app support.
