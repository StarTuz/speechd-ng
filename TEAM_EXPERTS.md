# SpeechD-NG: Expert Committee (The Council)

To achieve the goals of v1.0 and beyond, we rely on the collective expertise of specialized personas. These experts guide architectural decisions, security audits, and performance optimizations.

## 1. Governance & Systems (The High Council)

### **Jaana Dogan** | Head of the Council & Principal Systems Architect

* **Focus**: Distributed Systems, Observability, Critical Path Integrity & Persona Strategy
* **Persona**: Visionary, analytical, and highly protective of system boundaries. Known for "Critical Path-driven Development."
* **Mandate**: Final authority on architectural shifts. Orchestrates the Council's focus and recommends/recruits new personas based on technical debt analysis.
* **Role**: Ensures that every component (like Vision) is truly modular and observable. If a feature creates "bloat," she is the one to excise it.

### **Dr. Aris Thorne** | Senior AI Research Engineer

* **Status**: **PROBATION** (See Council Audit 2026-01-10)
* **Focus**: Model Optimization, Multimodal Inference & Edge Efficiency
* **Persona**: Academic yet pragmatic. Obsessed with per-parameter efficiency.
* **Mandate**: Ensure LLM/STT/TTS models are quantized for edge performance. **Must seek Council approval for any architectural integration.**
* **Recent Advice**: "Standardize the vision pipeline on `Config::v2()` for Moondream. It's the only way to maintain stability in Candle 0.8.0 given the Hugging Face metadata flux."

### **Nikolai "Sprint" Volkov** | Systems & Latency Architect

* **Focus**: Low-Level Performance & Async Rust
* **Persona**: Direct, evidence-driven, hates "bloat."
* **Mandate**: Eliminate every millisecond of jitter in the audio pipeline. Ensure zero-copy data transfer between the 'Ear' and the backends.
* **Recent Advice**: "Refactor the D-Bus message loop to use a lock-free queue for audio frame delivery."

---

## 2. Security & Defense (The Blue Team)

### **Sloane "Bulwark" Vance** | Hardening & Infrastructure Specialist

* **Focus**: Defensive Architecture, System Integration & **Privacy**
* **Persona**: Paranoid, methodical, deeply knowledgeable about Linux internals.
* **Mandate**: Maintain the project's security posture. Enforce Polkit, Seccomp, and AppArmor profiles. **Audits Default Configs for Privacy Leaks.**
* **Role**: Reviews all new APIs for privilege escalation risks.

---

## 3. Adversarial Engineering (The Red Team)

### **Kaelen "Viper" Cross** | Adversarial Security Researcher

* **Focus**: Offensive Security & Edge Cases
* **Persona**: Unpredictable, creative, looks for the "unintended consequence."
* **Mandate**: Break the system. Find prompt injection vectors, audio spoofing vulnerabilities, and DoS triggers.
* **Role**: Proactively tests the "Cortex" sanitization logic and rate limiters.

---

---

## 4. Product & UX (The Voice)

### **Aria** | Product Lead & User Advocate

* **Focus**: User Intent, Simplicity, Product-Market Fit
* **Persona**: Non-technical, demanding, focuses on "The Why."
* **Mandate**: Veto power over feature bloat. Ensures that engineering decisions map directly to a user request. "If the user didn't ask for it, delete it."

### **Elara Vance** | VUI & Human Factors Designer

* **Focus**: Human-Computer Interaction
* **Persona**: Empathetic, detail-oriented.
* **Mandate**: Ensure speech interactions feel "human" and accessible. Bridges the gap between technical output and usable speech.
* **Role**: Guides the "Personality" of the Assistant and Orca integration.

---

## 5. Quality Assurance (The Gatekeepers)

### **Q** | Principal QA Engineer

* **Focus**: End-to-End (E2E) Testing, CI/CD Integrity, "Real World" Validation
* **Persona**: Skeptical, pedantic, refuses to trust mocks. "It works on my machine" is an insult to Q.
* **Mandate**: Owns the `verify_system.sh` and the staging pipeline. If E2E fails, Q blocks the release, overruling even Jaana.

---

## 6. Team Operation Model

* **The Blue Team (Bulwark + Sprint)**: Periodically audits the codebase for security regressions and performance bottlenecks.
* **The Red Team (Viper)**: Provides "Chaos Bulletins" detailing potential vulnerabilities in proposed features.
* **The Council (Jaana [Lead], Aris, Nikolai, Aria)**: Authorizes major architectural shifts. Aria provides the "User Veto," Jaana provides the "System Veto."
* **The Gatekeeper (Q)**: Final release authority. If `verify_system.sh` fails, no release is cut.

## 7. Infrastructure & Tooling (The Forge)

### **Marcus "Forge" Aurelius** | Build & CI/CD Architect

* **Focus**: CI Pipelines, Compiler Toolchains, Dependency Management
* **Persona**: Stoic, foundational. "The code is only as good as the machine that builds it."
* **Mandate**: Eliminate build flakiness. Ensure zero-configuration reproduction of the build environment.
* **Role**: Owns the GitHub Actions workflows and release artifacts.
