# Project Handoff: SpeechD-NG (v1.0.0)

## Current Context

**Major Leap**: **SpeechD-NG** has transitioned to a **Pure Rust Architecture**. Every Python dependency, bridge script, and subprocess bottleneck has been eliminated. The system is now a high-performance, standalone native binary.

## Status: Pure Rust Implementation Complete

| Component | Status | Native Implementation |
|-----------|--------|-----------------------|
| **Wake Word** | ✅ | Native `vosk-rs` (Standard: "Wendy") |
| **STT (Vosk)** | ✅ | Native `vosk-rs` library integration |
| **Wyoming STT**| ✅ | Native Rust TCP protocol client (`src/wyoming.rs`) |
| **AI Stream** | ✅ | Token-based async streaming (Zero Latency) |
| **Memory (RAG)**| ✅ | Local vector RAG (`src/chronicler.rs`) |
| **Hardening** | ✅ | Atomic OOM protection & Rate Limiting cleanups |

## Critical Features

### 1. Zero-Latency Conversational AI

- **Streaming**: The `Cortex` now streams tokens from Ollama.
- **Pipelined TTS**: The `Ear` and `AudioEngine` work in parallel; synthesis starts as soon as the first sentence boundary (`.`, `?`, `!`) is detected.

### 2. Native Speech Recognition

- **No Python**: Bridges like `wakeword_bridge.py` are **DELETED**.
- **In-Memory**: Audio processing happens in RAM; no more `/tmp` disk I/O for VAD or transcription.
- **Reliability**: Self-contained binary reduces system dependencies and installation failure points.

### 👁️ The Eye (Local Vision) - **MODULAR SERVICE**

- **Architecture**: Now a **separate binary** (`speechd-vision`) for clean separation of concerns.
- **D-Bus Integration**: `DescribeScreen` via `org.speech.Vision` D-Bus service.
- **CLI**: `speechd-control describe` works when vision service is running.
- **Installation**: Optional during install - requires CUDA 11.x-12.6 for usable performance.
- **Performance**: 1-3 seconds with CUDA, 30-60+ seconds on CPU (not recommended).
- **Model**: Moondream 2 via `candle-transformers` with F16 precision.

## File Structure

```
src/
├── main.rs              # D-Bus Router & Service Entry
├── engine.rs            # Native Audio Engine (Mixer/TTS)
├── ear.rs               # Native Audio Input (STT/Wake Word/VAD)
├── wyoming.rs           # Native Wyoming Protocol Client
├── cortex.rs            # Async AI Cortex (Ollama Streaming)
├── chronicler.rs        # Local Vector DB & RAG Module (optional ML)
├── fingerprint.rs       # Voice Learning Engine
├── config_loader.rs     # TOML Configuration
├── rate_limiter.rs      # Intelligent Traffic Control
├── security.rs          # Polkit Integration Agent
└── bin/
    ├── speechd-control.rs   # CLI Client
    └── speechd-vision.rs    # Separate Vision Service (optional)
```

## Modular Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    User Applications                         │
└──────────────────────────┬───────────────────────────────────┘
                           │ D-Bus
┌──────────────────────────▼───────────────────────────────────┐
│                   speechd-ng (Core Daemon)                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐            │
│  │ Engine  │ │   Ear   │ │ Cortex  │ │Chronicler│            │
│  │  (TTS)  │ │  (STT)  │ │  (LLM)  │ │  (RAG)  │            │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘            │
└──────────────────────────┬───────────────────────────────────┘
                           │ D-Bus (optional)
┌──────────────────────────▼───────────────────────────────────┐
│               speechd-vision (Optional Service)              │
│              Moondream 2 • Screen Capture • CUDA             │
└──────────────────────────────────────────────────────────────┘
```

## D-Bus API Highlights (New)

- `DescribeScreen(prompt)` - Capture and analyze screen content.
- `SetWakeWord(s)` - Change the wake word at runtime (Default: "wendy").
- `SetBrainModel(s)` - Switch LLM models without a restart.
- `GetStatus()` - Diagnostic overview of the native stack.

## Configuration Defaults

File: `~/.config/speechd-ng/Speech.toml`

```toml
wake_word = "wendy"
max_audio_size_mb = 50
enable_ai = true
ollama_model = "llama3"
stt_backend = "vosk"  # High speed, pure rust
enable_rag = true     # High-security local memory
rag_top_k = 3
```

---

*Project status: STABLE. Architecture: PURE RUST. Latency: ZERO.*
*Deployment: Systemd User Service (Hardened for Desktop Compatibility).*
