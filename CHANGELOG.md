# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-01-09

### Added

- **Multimodal "Eye"**: Local Computer Vision integration for screen analysis.
- **"Cortex" Brain**: Local LLM integration (Ollama) for context-aware responses.
- **"Chronicler" Memory**: Vector database (Sled + BERT) for long-term conversation retention (RAG).
- **"Wendy" Wake Word**: Native, hands-free wake word detection.
- **Neural TTS**: First-class support for Piper neural voices.
- **Local STT**: Offline speech-to-text via Vosk or Wyoming.
- **Voice Learning**: Fingerprinting system to learn and correct user-specific speech patterns.
- **DBus API**: Comprehensive IPC interface for external control and integration.

### Fixed

- **CI Pipeline**: Resolved linker issues with `libvosk` and stress test timeouts.
- **Security**: Hardened Systemd sandboxing and Polkit integration.
- **Stability**: Resolved concurrency collisions in audio engine.

### Verified

- **Council Stress Tests**: Passed Adversarial Image, Chronicler Flooding, and Concurrency Collision protocols.
