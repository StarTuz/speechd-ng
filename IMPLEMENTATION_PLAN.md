# SpeechD-NG Implementation Plan: Voice Learning Enhancements

This document outlines the next phases of development for SpeechD-NG, focused on advanced voice learning features inspired by TuxTalks.

## Current Status

**Phases 1-12**: ✅ Complete
- Core D-Bus service, TTS engines, STT, LLM integration, wake word, passive learning, **manual training**, **import/export**, **ignored commands**, **VAD**

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

## Phase 10: Pattern Import/Export ✅ COMPLETE

**Goal**: Share learned voice patterns between systems or back up learned data.

**Value**: Medium | **Effort**: Low | **Priority**: Easy Win

### Overview
Fingerprint data is already stored as JSON. Expose D-Bus methods to export/import this data.

### Implementation Steps

#### 10.1 Export Method ✅
- [x] `ExportFingerprint(path: String) -> bool`
  - Exports fingerprint data to specified path
  - Returns success status

#### 10.2 Import Method ✅
- [x] `ImportFingerprint(path: String, merge: bool) -> u32`
  - If `merge=true`: Adds patterns from file to existing (doesn't overwrite)
  - If `merge=false`: Replaces current fingerprint entirely
  - Returns count of patterns after import

#### 10.3 Pattern Stats ✅ (Already implemented in Phase 9)
- [x] `GetFingerprintStats() -> (manual_count, passive_count, command_count)`
  - Quick overview of fingerprint status

#### 10.4 Additional Helper ✅
- [x] `GetFingerprintPath() -> String`
  - Returns path to fingerprint data file

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

## Phase 11: Ignored Commands Tracking ✅ COMPLETE

**Goal**: Track failed/unrecognized commands for later manual correction.

**Value**: Medium | **Effort**: Low | **Priority**: Debugging Helper

### Overview
When the LLM can't resolve an ASR transcription to a meaningful command, store it for later review. Users or GUI tools can then manually add corrections.

### Implementation Steps

#### 11.1 Track Failures in Fingerprint ✅
- [x] Add `ignored_commands: Vec<IgnoredCommand>` to FingerprintData
- [x] `IgnoredCommand { heard: String, timestamp: String, context: String }`
- [x] Cap at 50 most recent
- [x] Duplicate detection (don't add same command twice)

#### 11.2 API Methods ✅
- [x] `GetIgnoredCommands() -> Vec<(heard: String, timestamp: String, context: String)>`
- [x] `ClearIgnoredCommands() -> u32` (returns count cleared)
- [x] `CorrectIgnoredCommand(heard: String, meant: String) -> bool`
  - Removes from ignored list and adds as manual correction
- [x] `AddIgnoredCommand(heard: String, context: String)`
  - For testing/debugging

#### 11.3 Integration ✅
- [x] Cortex auto-adds commands when LLM returns "confused" or error responses
- [x] Fingerprint auto-saves ignored commands with timestamps

### Files to Modify
- `src/fingerprint.rs` - Add IgnoredCommand struct and methods
- `src/cortex.rs` - Report failures to fingerprint
- `src/main.rs` - Add D-Bus interface methods

---

## Phase 12: Improved Voice Activity Detection (VAD) ✅ COMPLETE

**Goal**: Smarter speech detection for more natural listening experience.

**Value**: Medium | **Effort**: Medium | **Priority**: Polish

### Overview
Replace fixed 4-second recording with energy-based VAD that starts when speech is detected and stops after silence.

### Implementation Steps

#### 12.1 VAD Parameters (Configurable) ✅
```toml
# Speech.toml
vad_speech_threshold = 500      # Energy level to detect speech start
vad_silence_threshold = 400     # Energy level to detect silence
vad_silence_duration_ms = 1500  # How long to wait before ending
vad_max_duration_ms = 15000     # Maximum recording length
```

#### 12.2 Implement in Ear Module ✅
- [x] Add `record_with_vad()` function alongside `record_and_transcribe()`
- [x] Calculate RMS energy per 10ms audio chunk
- [x] State machine: WAITING → SPEAKING → SILENCE_DETECTED → DONE
- [x] Configurable thresholds via Settings

#### 12.3 Wake Word Mode ✅
- [x] Autonomous mode now uses `record_with_vad()`
- [x] Waits for speech start (doesn't record silence)
- [x] Ends recording when user stops speaking naturally
- [x] Says "I didn't hear anything" if no speech detected

#### 12.4 D-Bus API ✅
- [x] `ListenVad()` - VAD-based listening method

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
| 10 | Pattern Import/Export | 1 hour | ✅ Complete |
| 11 | Ignored Commands | 1-2 hours | ✅ Complete |
| 12 | Improved VAD | 2-3 hours | ✅ Complete |
| 13 | Wyoming Protocol | 4-6 hours | ✅ Complete |

---

## Getting Started

To begin Phase 9:
```bash
cd /home/startux/Code/speechserverdaemon
# Review fingerprint module
cat src/fingerprint.rs
```

---

## Phase 15: Streaming Media Player (TBD)

**Goal**: Play audio from URLs directly via D-Bus, bypassing client-side player management.

**Value**: Medium | **Effort**: Medium

### Requirements
- `PlayAudio(url: String)` method
- Support for playback status events (Started, Finished)
- Volume control and cancellation

---

## Phase 16: Multi-Channel Support (TBD)

**Goal**: Support directed audio output for aviation headsets (COM1 vs COM2 separation).

**Value**: High (for flight sim) | **Effort**: Medium

### Requirements
- `SpeakChannel(text: String, voice: String, channel: String)`
- `channel` options: "default", "left", "right", "headset", "speakers"

---

*Last Updated: 2025-12-28*
