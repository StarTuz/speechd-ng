# Project Handoff: SpeechD-NG

## Current Context

We have completed **Phase 11** of the roadmap. **SpeechD-NG** is now a fully-featured, self-improving, hands-free speech assistant for Linux with comprehensive voice learning capabilities.

## Status: Phase 11 Completed (Ignored Commands Tracking)

### Completed Phases

| Phase | Feature | Status |
|-------|---------|--------|
| 1-8 | Core (D-Bus, TTS, STT, LLM, Wake Word, Passive Learning) | ✅ |
| 9 | Manual Voice Training API | ✅ |
| 10 | Pattern Import/Export | ✅ |
| 11 | Ignored Commands Tracking | ✅ |
| 12 | Improved VAD | 📋 Planned |
| 13 | Wyoming Protocol | 📋 Future |

## Functional Features

### TTS & Speech
-   **Neural TTS (Piper)**: High-quality voices with zero-config downloading
-   **Legacy TTS (eSpeak)**: Fast fallback synthesis
-   **Voice Selection**: Per-request voice selection via D-Bus
-   **SSIP Shim**: Legacy Orca compatibility (TCP 6560)

### AI & Context
-   **The Cortex**: Ollama integration for contextual understanding
-   **Speech Memory**: Rolling history of spoken text
-   **Query API**: Ask questions about recent speech

### Wake Word & Autonomous Mode
-   **Background Listener**: Vosk-powered wake word detection
-   **Command Loop**: STANDBY → WAKE → CAPTURE → THINK → RESPOND
-   **Configurable**: Custom wake word via config

### Voice Learning (Phases 8-11)
-   **Passive Learning**: Auto-learns from LLM corrections
-   **Manual Training**: Explicit word training via D-Bus
-   **Pattern Management**: Import/export learned patterns
-   **Ignored Commands**: Track and correct failed ASR attempts

## D-Bus API Summary

### Service Details
| Property | Value |
|----------|-------|
| Bus | Session |
| Service | `org.speech.Service` |
| Path | `/org/speech/Service` |
| Interface | `org.speech.Service` |

### Available Methods

**TTS:**
- `Speak(text)` - Speak with default voice
- `SpeakVoice(text, voice)` - Speak with specific voice
- `ListVoices()` - List installed voices
- `ListDownloadableVoices()` - List available downloads
- `DownloadVoice(voice_id)` - Download a voice

**AI:**
- `Think(query)` - Ask the AI about speech context
- `Listen()` - Record and transcribe

**Training (Phase 9):**
- `AddCorrection(heard, meant)` - Add correction pattern
- `TrainWord(expected, duration)` - Record and learn
- `ListPatterns()` - View all patterns
- `GetFingerprintStats()` - Get learning stats

**Import/Export (Phase 10):**
- `ExportFingerprint(path)` - Export patterns to file
- `ImportFingerprint(path, merge)` - Import patterns
- `GetFingerprintPath()` - Get fingerprint file path

**Ignored Commands (Phase 11):**
- `GetIgnoredCommands()` - List failed ASR attempts
- `CorrectIgnoredCommand(heard, meant)` - Fix and learn
- `ClearIgnoredCommands()` - Clear all ignored
- `AddIgnoredCommand(heard, context)` - Manual add

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
├── backends/
│   ├── mod.rs           # Backend trait
│   ├── piper.rs         # Piper neural TTS
│   └── espeak.rs        # eSpeak-ng TTS
├── ssip.rs              # Legacy Orca shim
└── wakeword_bridge.py   # Python/Vosk wake word

systemd/
└── speechd-ng.service   # Systemd user service

docs/
├── API_REFERENCE.md     # Complete D-Bus API docs
└── ANALYSIS.md          # Technical analysis
```

## Configuration

File: `~/.config/speechd-ng/Speech.toml`

```toml
# LLM
ollama_url = "http://localhost:11434"
ollama_model = "llama3"

# TTS
piper_model = "en_US-lessac-medium"
piper_binary = "piper"
tts_backend = "piper"

# Memory
memory_size = 50
enable_audio = true

# Wake Word
wake_word = "mango"
enable_wake_word = false
```

## Quick Test Commands

```bash
# Speak
busctl --user call org.speech.Service /org/speech/Service org.speech.Service Speak s "Hello"

# Add correction
busctl --user call org.speech.Service /org/speech/Service org.speech.Service AddCorrection ss "mozurt" "mozart"

# View patterns
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ListPatterns

# View stats
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetFingerprintStats

# Export patterns
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ExportFingerprint s "$HOME/Documents/patterns.json"

# View ignored commands
busctl --user call org.speech.Service /org/speech/Service org.speech.Service GetIgnoredCommands

# Correct ignored command
busctl --user call org.speech.Service /org/speech/Service org.speech.Service CorrectIgnoredCommand ss "plae musik" "play music"
```

## Known Limitations

-   **Microphone Exclusivity**: Wake word listener may conflict with other apps using exclusive mic access.
-   **Vosk Model Path**: Wake word bridge expects models in `~/.cache/vosk/`.
-   **Piper Binary Conflict**: If `/usr/bin/piper` exists (GTK pipe viewer), set explicit `piper_binary` path.
-   **Export Paths**: Due to sandboxing, exports only work to `~/.local/share/speechd-ng/` or `~/Documents/`.

## Next Steps (Phase 12+)

1. **Improved VAD**: Energy-based voice activity detection for natural conversation
2. **Wyoming Protocol**: Remote Whisper server support for better accuracy

## Repository

-   **GitHub**: https://github.com/StarTuz/speechd-ng
-   **Branch**: `main`
-   **Last Updated**: 2025-12-27
