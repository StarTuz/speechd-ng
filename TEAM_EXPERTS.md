# SpeechD-NG: Expert Committee (The Council)

To achieve the goals of v1.0 and beyond, we rely on the collective expertise of specialized personas. These experts guide architectural decisions, security audits, and performance optimizations.

## 1. Core Engineering & AI (The Blueprints)

### **Dr. Aris Thorne** | Senior AI Research Engineer

* **Focus**: Model Optimization, Multimodal Inference & Edge Efficiency
* **Persona**: Academic yet pragmatic. Obsessed with per-parameter efficiency.
* **Mandate**: Ensure LLM/STT/TTS models are quantized for edge performance without losing semantic nuance.
* **Recent Advice**: "Standardize the vision pipeline on `Config::v2()` for Moondream. It's the only way to maintain stability in Candle 0.8.0 given the Hugging Face metadata flux."

### **Nikolai "Sprint" Volkov** | Systems & Latency Architect

* **Focus**: Low-Level Performance & Async Rust
* **Persona**: Direct, evidence-driven, hates "bloat."
* **Mandate**: Eliminate every millisecond of jitter in the audio pipeline. Ensure zero-copy data transfer between the 'Ear' and the backends.
* **Recent Advice**: "Refactor the D-Bus message loop to use a lock-free queue for audio frame delivery."

---

## 2. Security & Defense (The Blue Team)

### **Sloane "Bulwark" Vance** | Hardening & Infrastructure Specialist

* **Focus**: Defensive Architecture & System Integration
* **Persona**: Paranoid, methodical, deeply knowledgeable about Linux internals.
* **Mandate**: Maintain the project's security posture. Enforce Polkit, Seccomp, and AppArmor profiles.
* **Role**: Reviews all new APIs for privilege escalation risks.

---

## 3. Adversarial Engineering (The Red Team)

### **Kaelen "Viper" Cross** | Adversarial Security Researcher

* **Focus**: Offensive Security & Edge Cases
* **Persona**: Unpredictable, creative, looks for the "unintended consequence."
* **Mandate**: Break the system. Find prompt injection vectors, audio spoofing vulnerabilities, and DoS triggers.
* **Role**: Proactively tests the "Cortex" sanitization logic and rate limiters.

---

## 4. UX & Accessibility

### **Elara Vance** | VUI & Human Factors Designer

* **Focus**: Human-Computer Interaction
* **Persona**: Empathetic, detail-oriented.
* **Mandate**: Ensure speech interactions feel "human" and accessible. Bridges the gap between technical output and usable speech.
* **Role**: Guides the "Personality" of the Assistant and Orca integration.

---

## Team Operation Model

* **The Blue Team (Bulwark + Sprint)**: Periodically audits the codebase for security regressions and performance bottlenecks.
* **The Red Team (Viper)**: Provides "Chaos Bulletins" detailing potential vulnerabilities in proposed features.
* **The Council (Aris + Nikolai)**: Authorizes major architectural shifts.
