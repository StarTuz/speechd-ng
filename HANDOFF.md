# Project Handoff: SpeechD-NG (v1.1.0)

## Current Context

**Major Leap**: **SpeechD-NG** has transitioned to a **Pure Rust Architecture**. Every Python dependency, bridge script, and subprocess bottleneck has been eliminated. The system is now a high-performance, standalone native binary.

## Status: Pure Rust Implementation Complete

| Component | Status | Native Implementation |
|-----------|--------|-----------------------|
| **Wake Word** | ✅ | Native `vosk-rs` (Standard: "Wendy") |
| **STT (Vosk)** | ✅ | Native `vosk-rs` library integration |
| **Wyoming STT**| ✅ | Native Rust TCP protocol client (`src/wyoming.rs`) |
| **AI Stream** | ✅ | Modular `BrainBackend` trait (Ollama + BitNet/OpenAI) |
| **Memory (RAG)**| ✅ | Local vector RAG (`src/chronicler.rs`) |
| **Hardening** | ✅ | Polkit Security, Atomic OOM protection & Rate Limiting |

## Critical Features

### 1. Modular AI Architecture (`BrainBackend`)

SpeechD-NG is now backend-agnostic for AI reasoning.

- **BrainBackend Trait**: Defines `prompt` and `stream` methods.
- **Backends**: Native support for **Ollama** and **BitNet/OpenAI** APIs.
- **Streaming**: Sentence-boundary synthesis started as soon as the first sentence is collected from the AI stream.

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
├── cortex.rs            # Async AI Cortex (Multi-Backend Dispatcher)
├── backends/            # Backend Modules
│   ├── mod.rs           # BrainBackend & SpeechBackend Traits
│   ├── ollama.rs        # Native Ollama AI Backend
│   ├── openai.rs        # OpenAI-Compatible Backend (BitNet, etc.)
│   ├── espeak.rs        # eSpeak-ng Audio Backend
│   ├── piper.rs         # Piper TTS Audio Backend
│   └── whisper.rs       # Whisper STT Backend
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
ai_backend = "ollama"  # or "bitnet"
bitnet_url = "http://localhost:8000"
bitnet_model = "models/bitnet_b1_58-3B"
stt_backend = "vosk"  # High speed, pure rust
enable_rag = true     # High-security local memory
rag_top_k = 3
```

---

## 🧪 Quality & Testing (New: Phase 21 & 22)

The system now includes a specialized AI validation layer to prevent regression during backend swaps.

### 1. One-Click Verification

Team members should run this after any change to the `cortex` or `backends` modules:

```bash
cargo test --test ai_integration
```

This mocks both **Ollama** and **OpenAI/BitNet** REST APIs, verifying trait integrity and stream parsing in <1s.

### 2. Comparative Benchmarking (Phase 22)

When evaluating new BitNet models vs Ollama baselines:

- **Protocol**: Refer to `BITNET_COMPARISON_PROTOCOL.md` for metrics.
- **Focus**: **TTFT** (Time To First Token) and **RSS Memory Footprint**.
- **Requirement**: Use `speechd-control monitor` to capture real-time performance data.

---

*Project status: STABLE. Architecture: PURE RUST. Latency: ZERO.*
*Deployment: Systemd User Service (Hardened for Desktop Compatibility).*
