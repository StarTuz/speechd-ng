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

## Phase 5: Plug-in & Premium Voice System (✅ Completed)
**Goal**: Extensibility & High Quality.
-   [x] **Piper TTS Integration**: Neural, human-like local voices.
-   [x] **Intelligent Backend Mixer**: Simultaneous support for eSpeak & Piper.
-   [x] **Rich Metadata Discovery**: Extraction of language/quality/gender from model configs.
-   [x] **Zero-Config Downloader**: Securely fetch neural models from Hugging Face via D-Bus.
-   [x] **Backend Trait**: Abstract backends for generic engine support.

## Phase 6: Input & Accessibility (✅ Completed)
**Goal**: Two-way interaction (The "Ears").
-   [x] **Microphone Stream**: Secure access to mic via `cpal`.
-   [x] **Speech-to-Text (STT)**: Integration with Whisper (CLI) and Vosk (CLI).
-   [x] **Orca Compatibility**: Shim to pretend to be `speech-dispatcher` for legacy app support.

## Phase 7: Hands-Free Interaction (✅ Completed)
**Goal**: Semi-Autonomous Operation.
-   [x] **Wake Word Detection**: Low-power standby observing for "StarTuz" via Vosk.
-   [x] **Voice Commands**: Trigger actions via speech (e.g., "Summarize the last 10 minutes").
-   [x] **Command Loop**: Seamless transition from standby to active listening and back.

## Phase 8: Personalized Voice Learning (✅ Completed)
**Goal**: Self-Improving Accuracy.
-   [x] **Fingerprint Module**: Local storage for voice patterns and command frequencies.
-   [x] **Passive Learning Loop**: Automatically learn from LLM-corrected transcription errors.
-   [x] **Contextual Prompt Injection**: Dynamically inject learned patterns into LLM system prompts for better interpretation.
-   [x] **Local Privacy**: All learning data remains private and local to the user's home directory.
