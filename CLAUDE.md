# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SpeechD-NG is a modern, secure, next-generation speech server for Linux written in pure Rust. It replaces `speech-dispatcher` with a D-Bus service (`org.speech.Service`) providing TTS, STT, AI reasoning, and voice learning. It runs as a systemd user service and is window-manager agnostic.

## Build Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Build with optional features
cargo build --release --features rag       # BERT embeddings for semantic search
cargo build --release --features vision    # Moondream 2 vision (includes rag)
cargo build --release --features cuda      # GPU acceleration (includes vision)

# Run tests
cargo test --verbose

# Run a specific test
cargo test test_name --verbose

# Lint
cargo clippy --all-targets --all-features

# Format
cargo fmt
```

**Note:** On systems where libvosk is installed via pip, you may need `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu` for tests. CI installs libvosk from the Python vosk package and copies the `.so` to `/usr/lib`.

## Architecture

### Binary Targets

- **`speechd-ng`** (`src/main.rs`) — Main daemon. Registers on D-Bus, initializes all subsystems.
- **`speechd-control`** (`src/bin/speechd-control.rs`) — CLI client (speak, listen, think, status).
- **`speechd-vision`** (`src/bin/speechd-vision.rs`) — Optional vision service, requires `vision` feature.

### Core Modules (`src/`)

| Module | Role |
|--------|------|
| `service.rs` | D-Bus interface implementation — all API methods live here (zbus `#[interface]`, one impl block) |
| `service_helpers/` | Logic extracted from `service.rs`: `guards.rs` (Polkit + rate limit), `brain.rs` (AI management), `audio_devices.rs` (wpctl parsing) |
| `engine.rs` | Audio output engine (actor pattern with message queue, uses rodio) |
| `ear.rs` | Audio input: microphone capture (cpal), VAD state machine, wake word detection |
| `cortex.rs` | AI dispatcher: async actor with message channel, short-term memory, prompt sanitization |
| `chronicler.rs` | Long-term memory: sled vector DB with optional BERT embeddings, fallback keyword matching |
| `fingerprint.rs` | Voice learning: stores correction patterns, passive learning from LLM |
| `config_loader.rs` | TOML config from `~/.config/speechd-ng/Speech.toml`, lazy_static `SETTINGS` global; also overrideable via `SPEECH_*` env vars |
| `error.rs` | `SpeechdError` enum — unified error type for all AI and backend operations |
| `rate_limiter.rs` | Per-sender token bucket rate limiting |
| `security.rs` | Polkit authorization for sensitive operations |
| `ssip.rs` | Speech Dispatcher protocol compatibility shim |
| `wyoming.rs` | Wyoming STT protocol client |
| `proactive.rs` | System monitoring, desktop notification listener |
| `context/` | Display server detection (X11/Wayland) |

### Backend Traits (`src/backends/mod.rs`)

Two core traits define the pluggable backend system:

- **`BrainBackend`** (async_trait): `prompt()` and `stream()` — implemented by `ollama.rs`, `openai.rs`, `fallback.rs`
- **`SpeechBackend`**: `synthesize()` and `list_voices()` — implemented by `piper.rs` (key: `"piper-tts"`), `espeak.rs`
- `whisper.rs` provides native Whisper STT

### Data Flow

1. D-Bus method call → `service.rs` → `service_helpers/guards.rs` (rate limit + Polkit check)
2. TTS: `service.rs` → `engine.rs` actor → `SpeechBackend` (Piper/eSpeak) → rodio playback
3. AI: `service.rs` → `cortex.rs` actor → `BrainBackend` (Ollama/OpenAI) → response
4. STT: `ear.rs` VAD → Vosk/Wyoming/Whisper → transcribed text
5. Memory: `cortex.rs` ↔ `chronicler.rs` (RAG retrieval before prompting)

### Configuration

Config lives at `~/.config/speechd-ng/Speech.toml`. Key settings: `ai_backend` (default `"bitnet"`; also `"ollama"` or `"auto"`), `tts_backend` (`"piper-tts"`/`"espeak"`), `stt_backend` (`"vosk"`/`"wyoming"`/`"whisper"`). Privacy features (`enable_microphone`, `enable_wake_word`) default to `false`; `enable_ai` defaults to `true`. Settings can also be overridden via `SPEECH_*` environment variables (e.g. `SPEECH_OLLAMA_URL`).

**No hot-reload.** `SETTINGS` is a `lazy_static! RwLock<Settings>` loaded once at startup. Config file changes require `systemctl --user restart speechd-ng`. There is no D-Bus `Reload` method yet (planned).

**piper-tts voice resolution** (`src/backends/piper.rs`): `piper_model` is read from live `SETTINGS` on every `speak()` call (so changes take effect after restart). Synthesizer tries exact filename match in `~/.local/share/piper/models/`, then falls back to first available `.onnx`+`.onnx.json` pair with a warning logged to stderr. No model installed → clear `NotFound` error with the install command. `piper_binary` is cached at daemon startup only.

## Guardrails (from GUARDRAILS.md)

These are **non-negotiable**:

- **Stability Doctrine**: Core daemon must stay compatible with LTS distros (Debian Stable/Ubuntu LTS). Bleeding-edge deps go in optional features/separate binaries only.
- **Zero-Recall Rule**: Dependency updates that require new system drivers are rejected for core.
- **One-Way Ratchet**: Privacy features (mic kill-switch, etc.) cannot be removed.
- **No Silent Failures**: Every error path must be logged or handled.
- Avoid manual edits to `Cargo.lock` unless resolving specific conflicts.
- Deletion of source files requires explicit verification first (Tier 3 action).
