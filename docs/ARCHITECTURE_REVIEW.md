# Architecture & Risk Assessment

## 1. Local LLM Dependency
**Risk**: High hardware requirements; potential failure on low-end devices.
**Mitigation**:
- **Design**: The system uses `Ollama` via HTTP. If the server is unreachable, the specific `Think` method returns a "Brain offline" error gracefully.
- **Resource Limits**: The `Cortex` memory is capped (default 50 items) to prevent infinite context growth.
- **Future Work**: Add explicit `enable_ai` flag in config to disable `Cortex` background loop entirely for low-power devices.

## 2. Privacy & Network Surface
**Policy**: "Offline First". The daemon assumes no internet access by default.

**External Connections (Audited):**
1.  **Ollama**: `http://localhost:11434` (User Configurable). Used for `Think` and `Passive Learning`.
2.  **Piper Voices (Hugging Face)**: `https://huggingface.co/rhasspy/...`
    - **Trigger**: ONLY when `DownloadVoice` D-Bus method is explicitly called by user/admin.
    - **Mitigation**: `systemd` config restricts `AF_INET` access. (Note: Domain filtering was removed due to systemd limitations, but firewall rules are recommended).
3.  **No Telemetry**: The daemon sends no usage data anywhere.

## 3. Performance & Wake Word
**Impact**: Continuous listening is CPU intensive.
**Current Status**:
- **VAD (Voice Activity Detection)**: Implemented (Phase 12) to only record when speech is detected.
- **Wake Word**: Uses `vosk-model-small`, which is lightweight (~50MB RAM).
- **CPU Usage**: Observed ~5-10% usage on a single core during active listening.
- **Future Work**: Benchmark on Raspberry Pi 4/5. 
- **Recommendation**: Use hardware-accelerated VAD (e.g., on-device DSP) for embedded targets to reduce CPU load.

## 4. Security
**Layering**:
1.  **D-Bus**: All methods are exposed on the Session Bus.
2.  **Polkit**: Sensitive methods (`TrainWord`, `DownloadVoice`, `Listen`?) check `org.speech.service.*` permissions.
3.  **Systemd Sandbox**:
    - `ProtectSystem=strict`
    - `ProtectHome=read-only` (Whitelisted: `~/.local/share/speechd-ng`, `~/Documents`)
    - `PrivateTmp=true`

## 5. ASR Poisoning (Passive Learning)
**Risk**: If the LLM hallucinates a correction (e.g., "You said X" when you said "Y"), it might enforce a bad pattern.
**Mitigation**:
- **Confidence**: Passive patterns are marked with significantly lower confidence than manual training.
- **Export/Import**: Users can backup their fingerprint file.
- **Auditing**: All passive corrections are logged. Confidence thresholds are configurable (`passive_confidence_threshold`).
- **Future Work**: `RollbackLastCorrection` method; UI for pattern review.

## 6. Compatibility
**Goal**: WM Agnostic.
**Status**:
- Works on TTY (tested via `systemctl --user`).
- Works on GNOME/KDE (via standard Session Bus).
- **SSIP Shim**: Implemented to emulate `speech-dispatcher` for ORCA/Firefox compatibility. (Requires further validation).

## 7. Action Items (Future Roadmap)
- [ ] Create `.deb` / `.rpm` packaging scripts.
- [ ] Add `enable_ai = false` config option. (✅ Completed)
- [ ] Implement `Rollback` and specialized "Safety" UI.
- [ ] Benchmark suite for latency measurements.
- [ ] **CI**: Add explicit offline-mode test (network disabled, Ollama unreachable) to verification pipeline.
