# Development Roadmap

This document outlines the strategic phases for SpeechD-NG development.

## Phase 1: Foundation (✅ Completed)
**Goal**: Establish the D-Bus presence and process architecture.
-   [x] Initialize Rust project.
-   [x] Implement `zbus` for `org.speech.Service`.
-   [x] Create Systemd service files.
-   [x] Verify IPC via `busctl`.

## Phase 2: Audio Engine (✅ Completed)
**Goal**: Reliable, non-blocking text-to-speech.
-   [x] Integrate `espeak-ng` via CLI/Bindings.
-   [x] Integrate `rodio` for audio playback.
-   [x] Implement Threaded Actor model (isolating audio from main loop).
-   [x] Handle "firehose" requests (queueing).

## Phase 3: The Cortex ("Brain & Body") (✅ Completed)
**Goal**: Centralize Intelligence in the Daemon, expose it via API.
-   [x] **Cortex Module**: Async actor holding the "Short-Term Memory" (Speech History).
-   [x] **Omniscient API**: D-Bus methods for clients to query context (e.g., `QueryContext`, `SummarizeHistory`).
-   [x] **Ollama Connector**: HTTP client to talk to `localhost:11434` for processing queries.
-   [ ] **Reference Client**: A simple CLI tool (simulating a WM widget) to demonstrate "Asking the Daemon" about what was said.

## Phase 4: Security & Configuration (✅ Completed)
**Goal**: Hardening and User Control.
-   [x] **Polkit Integration**: Security hook implemented (logs sender, enforcement via policy pending).
-   [x] **Configuration**: Load `Speech.toml` for customizable settings (Ollama URL, Memory Size, Audio Toggle).
-   [x] **Systemd Sandboxing**: 20+ security directives applied to service file.
-   [x] **LLM Prompt Sanitization**: User input filtered to prevent injection attacks.

## Phase 5: Plug-in & Premium Voice System (✅ Completed)
**Goal**: Extensibility & High Quality.
-   [x] **Piper TTS Integration**: Neural, human-like local voices.
-   [x] **Intelligent Backend Mixer**: Simultaneous support for eSpeak & Piper.
-   [x] **Rich Metadata Discovery**: Extraction of language/quality/gender from model configs.
-   [x] **Zero-Config Downloader**: Securely fetch neural models from Hugging Face via D-Bus.
-   [x] **Backend Trait**: Abstract backends for generic engine support.

## Phase 6: Input & Accessibility (✅ Completed)
**Goal**: Two-way interaction (The "Ears").
-   [x] **Microphone Stream**: Secure access to mic via `cpal`.
-   [x] **Speech-to-Text (STT)**: Integration with Whisper (CLI) and Vosk (CLI).
-   [x] **Orca Compatibility**: Shim to pretend to be `speech-dispatcher` for legacy app support.

## Phase 7: Hands-Free Interaction (✅ Completed)
**Goal**: Semi-Autonomous Operation.
-   [x] **Wake Word Detection**: Low-power standby observing for "StarTuz" via Vosk.
-   [x] **Voice Commands**: Trigger actions via speech (e.g., "Summarize the last 10 minutes").
-   [x] **Command Loop**: Seamless transition from standby to active listening and back.

## Phase 8: Personalized Voice Learning (✅ Completed)
**Goal**: Self-Improving Accuracy.
-   [x] **Fingerprint Module**: Local storage for voice patterns and command frequencies.
-   [x] **Passive Learning Loop**: Automatically learn from LLM-corrected transcription errors.
-   [x] **Contextual Prompt Injection**: Dynamically inject learned patterns into LLM system prompts for better interpretation.
-   [x] **Local Privacy**: All learning data remains private and local to the user's home directory.

---

## Future Phases (Planned)

> See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for detailed implementation steps.

## Phase 9: Manual Voice Training (✅ Completed)
**Goal**: Explicit user-driven voice training for problematic words.
-   [x] **TrainWord D-Bus Method**: Record user saying a word, learn ASR error patterns.
-   [x] **AddCorrection D-Bus Method**: Directly add correction without recording.
-   [x] **ListPatterns D-Bus Method**: List all learned patterns for debugging/UI.
-   [x] **GetFingerprintStats D-Bus Method**: Quick overview of learning status.
-   [x] **Training Feedback**: Speak confirmation of what was learned.

## Phase 10: Pattern Import/Export (✅ Completed)
**Goal**: Share and backup voice patterns.
-   [x] **ExportFingerprint**: Save learned patterns to file.
-   [x] **ImportFingerprint**: Load/merge patterns from file.
-   [x] **GetFingerprintPath**: Get path to fingerprint data file.

## Phase 11: Ignored Commands Tracking (✅ Completed)
**Goal**: Track failed commands for later correction.
-   [x] **IgnoredCommand Struct**: Tracked with heard, timestamp, context.
-   [x] **GetIgnoredCommands**: D-Bus API to retrieve failed ASR attempts.
-   [x] **CorrectIgnoredCommand**: Fix errors and add to fingerprint patterns.
-   [x] **ClearIgnoredCommands**: Clear all ignored commands.
-   [x] **Auto-Integration**: Cortex auto-adds commands when LLM is confused.

## Phase 12: Improved VAD (✅ Completed)
**Goal**: Smarter speech detection for natural conversation.
-   [x] **Energy-Based VAD**: Detect speech start/end by audio energy (RMS).
-   [x] **Configurable Thresholds**: Speech/silence levels, timeouts in config.
-   [x] **Natural Recording**: Record only when user is speaking.
-   [x] **ListenVad D-Bus Method**: New API for VAD-based listening.
-   [x] **Wake Word Integration**: Autonomous mode uses VAD.

## Phase 13: Wyoming Protocol (✅ Completed)
**Goal**: Remote ASR via Wyoming protocol for better accuracy.
-   [x] **Wyoming Client**: Stream audio to wyoming-whisper servers via `wyoming_bridge.py`.
-   [x] **Configurable Backend**: Switch between `vosk` and `wyoming` in config.
-   [x] **D-Bus Info API**: Get current backend info and status.

## Phase 14: Hardening & Packaging (✅ Completed)
**Goal**: Production readiness and distribution.
-   [x] **Packaging**: Create `.deb`, `.rpm`, and `Flatpak` manifests.
-   [x] **Safety**: Implement `RollbackLastCorrection` to undo bad learning.
-   [x] **Benchmarking**: Create latency/resource usage test suite.
-   [x] **CI Hardening**: Add offline-mode verification tests.

## Phase 15: Streaming Media Player (✅ Completed)
**Goal**: Play audio from URLs directly via D-Bus.
-   [x] **PlayAudio D-Bus Method**: Download and play audio from HTTP/HTTPS URLs.
-   [x] **StopAudio D-Bus Method**: Cancel current audio playback.
-   [x] **SetVolume / GetVolume**: Configurable playback volume (0.0-1.0).
-   [x] **GetPlaybackStatus**: Query current playback state and URL.
-   [x] **Configurable Limits**: Max file size (50MB default), download timeout.

## Phase 16a: Multi-Channel Audio (✅ Completed)
**Goal**: Route audio to different stereo channels for aviation/gaming use cases.
-   [x] **SpeakChannel D-Bus Method**: Speak to left, right, center, or stereo.
-   [x] **PlayAudioChannel D-Bus Method**: Play URL audio to specific channel.
-   [x] **ListChannels D-Bus Method**: List available channel options.
-   [x] **Use Case: Aviation**: COM1 to left ear, COM2 to right ear.
-   [x] **Use Case: Gaming**: Voice chat vs game audio separation.

## Phase 16b: PipeWire Device Routing (✅ Completed)
**Goal**: Route audio to specific PipeWire output devices.
-   [x] **ListSinks D-Bus Method**: Enumerate available audio sinks.
-   [x] **GetDefaultSink D-Bus Method**: Get current default sink.
-   [x] **SpeakToDevice D-Bus Method**: Route TTS to specific device by ID.
-   [x] **Use Case: Multi-device**: Headset for voice, speakers for ambient.
-   [x] **Use Case: Home Automation**: Announce to kitchen vs living room.

## Phase 17a: Polkit Enforcement (✅ Completed)
**Goal**: Real PolicyKit authorization for sensitive D-Bus methods.
-   [x] **zbus_polkit Integration**: Real CheckAuthorization calls to PolicyKit.
-   [x] **Policy File**: `org.speech.service.policy` with action definitions.
-   [x] **Protected Methods**: DownloadVoice, Think, Listen, ListenVad, TrainWord.
-   [x] **Desktop Sessions**: Auto-approve for active desktop users.
-   [x] **Remote/Inactive**: Require admin authentication.

