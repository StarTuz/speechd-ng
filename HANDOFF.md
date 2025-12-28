# Project Handoff: SpeechD-NG

## Current Context

We have completed **Phase 14 (Partial)** of the roadmap. **SpeechD-NG** is now a fully-featured, self-improving, hands-free speech assistant that is **architecture-hardening ready**.

## Status: Phase 13 Completed (Wyoming) / Phase 14 In Progress (Hardening)

### Completed Phases

| Phase | Feature | Status |
|-------|---------|--------|
| 1-11 | Core, AI, Training, Ignored Commands | ✅ |
| 12 | Improved VAD (Voice Activity Detection) | ✅ |
| 13 | Wyoming Protocol (Remote ASR) | ✅ |
| 14 | Hardening & Packaging | ✅ |

## Functional Features

### TTS & Speech
-   **Neural TTS (Piper)**: High-quality voices with zero-config downloading
-   **Legacy TTS (eSpeak)**: Fast fallback
-   **SSIP Shim**: Orca compatibility

### AI & Context
-   **The Cortex**: Ollama integration (with `enable_ai` toggle)
-   **Speech Memory**: Rolling history
-   **Voice Learning**: Manual training, Pattern Import/Export
-   **Safety**: Explicit `Rollback` of bad learning, Configurable passive confidence

### Listening & VAD (Phase 12)
-   **Energy-Based VAD**: Detects speech vs silence naturally
-   **Autonomous Mode**: Uses VAD for fluid conversation
-   **ListenVad API**: D-Bus method for VAD-based recording

### Wyoming Protocol (Phase 13)
-   **Architecture**: `src/wyoming_bridge.py` communicates with `wyoming-faster-whisper`
-   **Config**: `stt_backend = "wyoming"` enables streaming ASR to remote/local servers

## D-Bus API Summary

**Safety (Phase 14):**
- `RollbackLastCorrection()` - Undo the last learning event

**Configuration (Phase 13):**
- `GetSttBackend()` - Get current backend (vosk/wyoming)
- `GetWyomingInfo()` - Get host/port/model info

### Service Details
| Property | Value |
|----------|-------|
| Bus | Session |
| Service | `org.speech.Service` |
| Path | `/org/speech/Service` |
| Interface | `org.speech.Service` |

> **Full API Reference:** See [docs/API_REFERENCE.md](docs/API_REFERENCE.md)

## File Structure

```
src/
├── main.rs              # D-Bus interface & service startup
├── engine.rs            # Audio Engine (TTS mixer)
├── ear.rs               # Audio Input (STT, recording)
├── cortex.rs            # Memory & LLM (Ollama)
├── fingerprint.rs       # Voice Learning & Patterns
├── config_loader.rs     # Configuration management
├── security.rs          # Polkit hooks
├── backends/            # TTS Backends (Piper, eSpeak)
├── ssip.rs              # Legacy Orca shim
├── wakeword_bridge.py   # Python/Vosk wake word
└── wyoming_bridge.py    # Python/Wyoming bridge

examples/
└── python_client.py     # Reference implementation

docs/
├── ARCHITECTURE_REVIEW.md # Risk assessment & security audit
├── API_REFERENCE.md     # Complete D-Bus API docs
└── ANALYSIS.md          # Technical analysis
```

## Configuration

File: `~/.config/speechd-ng/Speech.toml`

```toml
# AI / LLM
enable_ai = true                    # Toggle Cortex features
passive_confidence_threshold = 0.1  # Threshold for auto-learning

# TTS
piper_model = "en_US-lessac-medium"
tts_backend = "piper"

# STT (Vosk or Wyoming)
stt_backend = "wyoming"
wyoming_host = "127.0.0.1"

# Wake Word
wake_word = "mango"
enable_wake_word = false
```

## Next Steps (Phase 14)

1.  **Packaging**: Create `.deb` / `.rpm` build scripts.
2.  **Benchmarking**: Measure latency on RPi 4.
3.  **CI Hardening**: Offline test suite.

## Repository

-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
-   **Last Updated**: 2025-12-27
