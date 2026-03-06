# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-03-05

### Added

- **Hot-reload config**: `speechd-control reload` and `systemctl --user reload speechd-ng` apply changes from `Speech.toml` at runtime without restarting the daemon. Rate limits, TTS backend, AI backend, and all other settings update immediately. Only `piper_binary` still requires a full restart.
- **Model-switching guide**: `docs/CLI_MANUAL.md` now covers TTS voice switching, BitNet model replacement, Ollama live model switching, and backend switching (bitnet/ollama/auto).

### Fixed

- **piper-tts voice fallback**: When the configured `piper_model` is not installed, the daemon now falls back to the first available model and logs a warning to stderr/journal instead of failing silently.
- **BitNet VRAM leak**: `llama-server` was auto-detecting the GPU and loading BitNet onto VRAM (~3 GB), providing no throughput benefit over CPU. Added `-ngl 0` to force CPU-only inference (~713 MB RAM, 0 VRAM).
- **piper-tts install.sh**: Auto-detects installed piper voice at install time instead of hardcoding `en_US-lessac-medium`.

### Tests

- Expanded from 49 to 63 tests. New coverage: piper-tts voice resolution (10 tests), `rate_limiter::update_limits()` (2 tests), `config_loader` write and reload-path validation (3 tests).
- Removed duplicate `test_error_display_backward_compat` from `tests/ai_integration.rs`.
- Removed unused `tests/common/` placeholder module.

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
