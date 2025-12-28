# Project Handoff: SpeechD-NG

## Current Context

We have completed **Phase 14** of the roadmap. **SpeechD-NG v0.2.0** is now a fully-featured, self-improving, hands-free speech assistant that is **production-hardened and package-ready**.

## Status: All Phases Completed (1-14)

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

**Diagnostics & Version (Phase 14):**
- `Ping()` - Diagnostic connectivity check (returns "pong")
- `GetVersion()` - Get daemon version (returns "0.2.0")
- `RollbackLastCorrection()` - Undo the last learning event

**Configuration (Phase 13):**
- `GetSttBackend()` - Get current backend (vosk/wyoming)
- `GetWyomingInfo()` - Get host/port/model info
- `GetStatus()` - Get diagnostic summary (ai_enabled, threshold, backend, patterns)

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

## Packaging

Pre-built packages available in `dist/`:

| Package | Format | For |
|---------|--------|-----|
| `speechserverdaemon_0.2.0-1_amd64.deb` | Debian | Ubuntu, Debian, Mint |
| `speechserverdaemon-0.2.0-1.x86_64.rpm` | RPM | Fedora, openSUSE, RHEL |
| `org.speech.Service-0.2.0.flatpak` | Flatpak | Universal Linux |

## Repository

-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
-   **Release**: v0.2.0
-   **Last Updated**: 2025-12-28
