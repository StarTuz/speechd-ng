# Architecture & Risk Assessment

## 1. Local LLM Dependency

**Risk**: High hardware requirements; potential failure on low-end devices.
**Mitigation**:

- **Design**: The system uses `Ollama` via HTTP. If the server is unreachable, the specialized `Think` method returns a "Brain offline" error gracefully.
- **Resource Limits**: The `Cortex` memory is capped (default 50 items).
- **Control**: `enable_ai` flag in config allows disabling LLM features entirely.
- **Robustness**: 30-second reasoning timeouts prevent false "offline" reports during VRAM cold-starts.

## 2. Privacy & Network Surface

**Policy**: "Offline First". The daemon assumes no internet access by default.

**External Connections (Audited):**

1. **Ollama**: `http://localhost:11434` (User Configurable). Used for `Think` and `Passive Learning`.
2. **Piper Voices (Hugging Face)**: `https://huggingface.co/rhasspy/...`
    - **Trigger**: ONLY when `DownloadVoice` D-Bus method is explicitly called.
3. **No Telemetry**: The daemon sends no usage data anywhere.

## 3. Performance & Wake Word

**Impact**: Continuous listening is CPU intensive.
**Current Status**:

- **VAD (Voice Activity Detection)**: Implemented (Phase 12) to only record when speech is detected.
- **Wake Word**: Uses `vosk-model-small`, which is lightweight (~50MB RAM).
- **CPU Usage**: Observed ~5-10% usage on a single core during active listening.

## 4. Security

**Layering**:

1. **D-Bus**: All methods are exposed on the Session Bus.
2. **Polkit**: Sensitive methods (`TrainWord`, `DownloadVoice`, `Listen`, `ManageBrain`, `Think`) check `org.speech.service.*` permissions.
3. **Systemd Sandbox**:
    - `ProtectSystem=strict`
    - `ProtectHome=read-only` (Whitelisted: `~/.local/share/speechd-ng`, `~/Documents`)
    - `PrivateTmp=true`

## 5. ASR Poisoning (Passive Learning)

**Risk**: hallucinated corrections might enforce bad patterns.
**Mitigation**:

- **Confidence**: Passive patterns have lower confidence than manual training.
- **Backup**: Users can backup their fingerprint file.
- **Auditing**: Configuration `passive_confidence_threshold` controls sensitivity.
- **Safety**: `RollbackLastCorrection` method allows undoing the last learning event.

## 6. Action Items (Technical Debt)

- [x] Create `.deb` / `.rpm` packaging scripts.
- [x] Add `enable_ai = false` config option.
- [x] Implement `Rollback` and specialized "Safety" UI.
- [x] Benchmark suite for latency measurements.
- [x] **CI**: Add explicit offline-mode test to verification pipeline.
- [x] **Management**: Native D-Bus control for AI service lifecycle (Phase 19).
