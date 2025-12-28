# SpeechD-NG Implementation Plan: Voice Learning Enhancements

This document outlines the next phases of development for SpeechD-NG, focused on advanced voice learning features inspired by TuxTalks.

## Current Status

**Phases 1-9**: ✅ Complete
- Core D-Bus service, TTS engines, STT, LLM integration, wake word, passive learning, **manual training**

---

## Phase 9: Manual Voice Training API ✅ COMPLETE

**Goal**: Allow users to explicitly train problematic words for higher accuracy.

**Value**: High | **Effort**: Medium | **Priority**: ⭐ Highest

### Overview
Users can teach the system words that ASR consistently mishears. Unlike passive learning (which requires LLM correction), manual training lets users directly associate ASR errors with intended words.

### Implementation Steps

#### 9.1 Extend Fingerprint Module ✅
- [x] Add `add_manual_correction(heard: String, meant: String)` method
- [x] Manual corrections get higher base confidence (0.7 vs 0.3 for passive)
- [x] Store source type: `"passive"` or `"manual"` per pattern

#### 9.2 Add D-Bus Training Methods ✅
- [x] `TrainWord(expected: String, duration_secs: u32) -> (heard: String, success: bool)`
  - Records audio for `duration_secs`
  - Transcribes using STT
  - Stores `heard → expected` mapping with high confidence
- [x] `AddCorrection(heard: String, meant: String) -> bool`
  - Direct API for adding corrections without recording
  - Useful for GUI tools or automated imports
- [x] `ListPatterns() -> Vec<(heard, meant, confidence_info)>`
  - List all learned patterns for debugging/UI
- [x] `GetFingerprintStats() -> (manual_count, passive_count, command_count)`
  - Quick overview of learning status

#### 9.3 Training Feedback ✅
- [x] Return what ASR actually heard for user verification
- [x] Speak confirmation: "I heard 'X'. I'll remember that means 'Y'."

### Files to Modify
- `src/fingerprint.rs` - Add manual correction method
- `src/main.rs` - Add D-Bus interface methods
- `src/ear.rs` - Add short recording function for training

### Testing
```bash
# Train a word
busctl --user call org.speech.Service /org/speech/Service org.speech.Service TrainWord su "abba" 3

# Direct add
busctl --user call org.speech.Service /org/speech/Service org.speech.Service AddCorrection ss "ever" "abba"
```

---

## Phase 10: Pattern Import/Export

**Goal**: Share learned voice patterns between systems or back up learned data.

**Value**: Medium | **Effort**: Low | **Priority**: Easy Win

### Overview
Fingerprint data is already stored as JSON. Expose D-Bus methods to export/import this data.

### Implementation Steps

#### 10.1 Export Method
- [ ] `ExportFingerprint(path: String) -> bool`
  - Copies `~/.local/share/speechd-ng/fingerprint.json` to specified path
  - Returns success status

#### 10.2 Import Method
- [ ] `ImportFingerprint(path: String, merge: bool) -> u32`
  - If `merge=true`: Adds patterns from file to existing (doesn't overwrite)
  - If `merge=false`: Replaces current fingerprint entirely
  - Returns count of patterns imported

#### 10.3 Pattern Stats ✅ (Already implemented in Phase 9)
- [x] `GetFingerprintStats() -> (manual_count, passive_count, command_count)`
  - Quick overview of fingerprint status

### Files to Modify
- `src/fingerprint.rs` - Add export/import/stats methods
- `src/main.rs` - Add D-Bus interface methods

### Testing
```bash
# Export
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ExportFingerprint s "/tmp/my_fingerprint.json"

# Import (merge mode)
busctl --user call org.speech.Service /org/speech/Service org.speech.Service ImportFingerprint sb "/tmp/shared_fingerprint.json" true
```

---

## Phase 11: Ignored Commands Tracking

**Goal**: Track failed/unrecognized commands for later manual correction.

**Value**: Medium | **Effort**: Low | **Priority**: Debugging Helper

### Overview
When the LLM can't resolve an ASR transcription to a meaningful command, store it for later review. Users or GUI tools can then manually add corrections.

### Implementation Steps

#### 11.1 Track Failures in Fingerprint
- [ ] Add `ignored_commands: Vec<IgnoredCommand>` to FingerprintData
- [ ] `IgnoredCommand { heard: String, timestamp: String, context: String }`
- [ ] Cap at 50 most recent

#### 11.2 API Methods
- [ ] `GetIgnoredCommands() -> Vec<(heard: String, timestamp: String)>`
- [ ] `ClearIgnoredCommands() -> bool`
- [ ] `CorrectIgnoredCommand(heard: String, meant: String) -> bool`
  - Adds correction and removes from ignored list

#### 11.3 Integration
- [ ] Cortex marks commands as "ignored" when LLM returns low confidence
- [ ] Fingerprint auto-saves ignored commands

### Files to Modify
- `src/fingerprint.rs` - Add IgnoredCommand struct and methods
- `src/cortex.rs` - Report failures to fingerprint
- `src/main.rs` - Add D-Bus interface methods

---

## Phase 12: Improved Voice Activity Detection (VAD)

**Goal**: Smarter speech detection for more natural listening experience.

**Value**: Medium | **Effort**: Medium | **Priority**: Polish

### Overview
Replace fixed 4-second recording with energy-based VAD that starts when speech is detected and stops after silence.

### Implementation Steps

#### 12.1 VAD Parameters (Configurable)
```toml
# Speech.toml
vad_speech_threshold = 500      # Energy level to detect speech start
vad_silence_threshold = 500     # Energy level to detect silence
vad_silence_duration_ms = 1500  # How long to wait before ending
vad_max_duration_ms = 10000     # Maximum recording length
```

#### 12.2 Implement in Ear Module
- [ ] Replace `record_and_transcribe(seconds)` with `record_until_silence()`
- [ ] Calculate RMS energy per audio chunk
- [ ] State machine: WAITING → SPEAKING → SILENCE_DETECTED → DONE

#### 12.3 Wake Word Mode
- [ ] After wake word, wait for speech start (don't record silence)
- [ ] End recording when user stops speaking naturally

### Files to Modify
- `src/ear.rs` - Implement VAD logic
- `src/config_loader.rs` - Add VAD settings

---

## Phase 13: Wyoming Protocol Support (Future)

**Goal**: Stream audio to remote Whisper servers for better accuracy.

**Value**: High | **Effort**: High | **Priority**: Future Phase

### Overview
The Wyoming protocol allows streaming audio to external ASR servers (like `wyoming-whisper`). This enables:
- GPU-accelerated transcription on a separate machine
- Larger/more accurate Whisper models
- Shared ASR infrastructure

### Implementation Steps
*(Detailed planning TBD when prioritized)*

#### 13.1 Wyoming Client
- [ ] Implement AsyncTcpClient wrapper
- [ ] AudioStart/AudioChunk/AudioStop events
- [ ] Transcript event handling

#### 13.2 Server Management
- [ ] Auto-start local wyoming-whisper if configured
- [ ] Health check and reconnection logic

#### 13.3 Configuration
```toml
stt_backend = "wyoming"  # or "vosk"
wyoming_host = "127.0.0.1"
wyoming_port = 10301
wyoming_auto_start = true
```

---

## Summary Timeline

| Phase | Feature | Est. Time | Status |
|-------|---------|-----------|--------|
| 9 | Manual Voice Training | 2-3 hours | ✅ Complete |
| 10 | Pattern Import/Export | 1 hour | 📋 Planned |
| 11 | Ignored Commands | 1-2 hours | 📋 Planned |
| 12 | Improved VAD | 2-3 hours | 📋 Planned |
| 13 | Wyoming Protocol | 4-6 hours | 📋 Future |

---

## Getting Started

To begin Phase 9:
```bash
cd /home/startux/Code/speechserverdaemon
# Review fingerprint module
cat src/fingerprint.rs
```

---

*Last Updated: 2025-12-27*
