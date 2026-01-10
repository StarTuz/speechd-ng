# Council Briefing: SpeechD-NG (v0.7.2)

The expert committee has completed its initial review of the SpeechD-NG codebase and infrastructure. Below are the definitive reports from each domain specialist.

---

## ⚡ Latency & Systems Report

**Expert**: Nikolai "Sprint" Volkov

### **Findings** (RESOLVED)

* **Serial Playback Bottleneck**: **FIXED** in `AudioEngine`. We now synthesize ahead of playback.
* **Disk I/O Overhead**: **FIXED**. Replaced `/tmp` writes with in-memory buffers and native Rust `vosk` processing.
* **Blocking HTTP**: **FIXED**. Ollama now uses async streaming streaming.

### **Proposals**

1. Implement a synthesis queue to pre-generate audio while current speech is active.
2. Switch to in-memory piping for STT to avoid `/tmp` disk writes.
3. Asynchronize the `PlayAudio` downloader to prevent engine stalls.

---

## 🧠 AI & Model Efficiency Report

**Expert**: Dr. Aris Thorne

### **Findings**

* **Context Loading**: **RESOLVED**. Implemented **Chronicler (Local RAG)**. The assistant now retrieves relevant historical context from a local vector store.
* **Prompt Construction**: The `Cortex` system prompt is clear but static. It could be more dynamic based on the current active window or user task.
* **Quantization**: We are currently dependent on external binary versions; shifting to GGUF 4-bit for local LLMs via `llama.cpp` bindings could stabilize performance.

### **Proposals**

1. Integrate `candle` or `llama.cpp` directly for more control over CPU/GPU offloading.
2. Develop a "Context Provider" plugin system to feed desktop environment metadata into the LLM prompt.

---

## 🛡 Security & Hardening Report

**Expert**: Sloane "Bulwark" Vance

### **Findings** (RESOLVED)

* **Polkit Coverage**: Coverage remains 100%.
* **Rate Limiter Leaks**: **FIXED**. `RateLimiter::cleanup` implemented.
* **Weak Sanitization**: **IMPROVED**. Basic hardening against multi-line escapes added.

### **Proposals**

1. Implement the `RateLimiter::cleanup` method and call it on a timer.
2. Replace string replacement sanitization with a proper structured prompt template or a dedicated sanitization library.

---

## 🐍 Adversarial Surface Report

**Expert**: Kaelen "Viper" Cross

### **Findings** (RESOLVED)

* **OOM Vulnerability**: **FIXED**. Global and per-request atomic memory counters implemented.
* **Escape Vectors**: Hardened prompt sanitization.
* **PID Spoofing**: Verified `zbus` credential passing.

### **Proposals**

1. Add a global "Total Memory Buffer" limit for the audio engine.
2. Implement homoglyph detection and more aggressive regex-based filtering for the Cortex input.

---

## 🎨 UX & Accessibility Report

**Expert**: Elara Vance

### **Findings**

* **Documentation Depth**: The D-Bus API reference is top-tier. Very easy for third-party devs to follow.
* **Documentation Depth**: The D-Bus API reference is top-tier. Very easy for third-party devs to follow.
* **Feedback Loops**: `TrainWord` provides great audio feedback ("I heard X, I'll remember it means Y"). This is a model for self-documenting features.
* **Discovery**: Multi-channel audio (5.1) is powerful but lacks an example "demo" script to show why users should care (e.g., spatial notifications).

### **Proposals**

1. Create a `spatial_demo.py` example script.
2. Simplify the "Phase Numbering" in public docs—it's useful for us, but confusing for external users.

---

## 🤖 Phase 4: Advanced AI Expansion (ACTIVE)

**Status**: Implementing Zero-Latency & Pure Rust

### **Objectives** (COMPLETED)

* **Zero-Latency Conversations**: **DONE**. Transitioned to **Token-Based Streaming**. TTS begins immediately.
* **Pure Rust Integration**: **DONE**. Eliminated all Python dependencies. **Native Vosk** and **Native Wyoming** protocol implemented.
* **OOM Protection**: **DONE**. Systems hardened against large audio payloads.
* **Chronicler RAG**: **DONE**. Local vector memory (Sled + Candle) implemented. Assitant has permanent local history.

---

## 🚀 Phase 6: Situational Awareness & Proactivity (NEXT)

**Expert Lead**: Nikolai "Sprint" Volkov & Dr. Aris Thorne

### **Objectives**

1. **Environmental Context**: Integrate `window_manager` hooks to feed active application and task info into the prompt.
2. **Proactive Speech**: Enable the daemon to initiate interactions (alerts, reminders, system updates) without a wake word.
3. **Synthesis Lookahead**: Implement token-predictive synthesis to reduce latency between "Brain query" and "Engine start".
4. **Multimodal Hooks**: Prepare the architecture for local Vision models (The Eye).
5. **Opt-in Proactivity**: Proactive sensors (system load, notifications) distinct from core loops, enabled only via user-intent (CLI/Config).
6. **API/CLI Vision Integration**: **DONE**. Expose `DescribeScreen` via D-Bus and added `describe` command to `speechd-control`.

---

## 👁️ Multimodal & Vision Report

**Expert**: Dr. Aris Thorne & Nikolai "Sprint" Volkov

### **Findings** (ACTIVE)

* **Infrastructure Readiness**: **COMPLETE**. Screenshot capture (X11/Wayland), D-Bus API, and CLI integration are all verified and functional.
* **Model Configuration Mismatch**: **Status**: **RESOLVED** via `Config::v2()` injection and `moondream1` stabilization.

- **Resolution**: Bypassed unstable Hugging Face `config.json` by using `candle-transformers` internal defaults. Fixed image preprocessing (378x378 resize + ImageNet normalization).
* **Verification**: `speechd-control describe` now returns accurate screen descriptions.

* **Revision instability**: Pinning to older revisions (`2024-03-06`, `2024-05-20`) resolves the `phi_config` issue but introduces `vocab_size` mismatches as the Moondream team iterates on the architecture.

### **Proposals / Request for Review**

1. **Manual Config Injection**: We should consider a local patch to the `Config` struct or a manually-crafted `config.json` that bridges the gap between the Hugging Face weights and the `candle` implementation.
2. **Stable Checkpoint Hosting**: It is requested that the council identify or host a verified "Candle-Safe" Moondream 2 checkpoint to prevent future breakage from upstream model updates.
3. **Rustup compatibility**: Ensure the `candle` dependencies remain pinned to 0.8.0 until a higher version stabilizes the Moondream config schema.
