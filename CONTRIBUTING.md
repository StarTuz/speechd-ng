# Contributing to SpeechD-NG

Thank you for your interest in contributing to SpeechD-NG! To maintain our standards of performance, security, and AI integrity, we follow a persona-based auditing process.

## Persona-Based Auditing

Before submitting a Pull Request, please consider how our **Expert Committee** (The Council) would evaluate your changes:

### 1. The Performance Check (Nikolai "Sprint" Volkov)

- Does this change introduce unnecessary allocations or synchronization locks?
- Could this be implemented using non-blocking async patterns?
- Have you verified that latency remains sub-50ms for the critical audio path?

### 2. The AI Efficiency Check (Dr. Aris Thorne)

- If modifying models or the Cortex, does it impact VRAM/CPU usage significantly?
- Is the prompt engineering robust against "hallucination" in the context of STT errors?

### 3. The Security Audit (Sloane "Bulwark" Vance)

- Does this new D-Bus method require Polkit authorization?
- are you following the "Principle of Least Privilege" for file system access?
- Have you checked for potential buffer overflows or injection vectors?

### 4. The Adversarial Test (Kaelen "Viper" Cross)

- Can an attacker bypass rate limits using this change?
- Does this feature introduce a new way to crash the daemon via malformed audio or text?

### 5. The Accessibility Review (Elara Vance)

- Does this change maintain compatibility with the SSIP shim for screen readers?
- Is the voice output natural and intuitive for human users?

## Guardrails

All code must strictly adhere to the requirements in **[GUARDRAILS.md](GUARDRAILS.md)**. Violations of "No-Touch Zones" or "Mandatory Verification" for high-risk actions will result in rejection.

## Process

1. **Open an Issue**: Discuss major changes before implementation.
2. **Follow Rust Idioms**: Use `cargo fmt` and `cargo clippy`.
3. **Document**: Update `API_REFERENCE.md` if adding or changing D-Bus methods.
4. **Self-Audit**: Perform the persona-based check listed above.

---

*Together, we build a faster, safer speech ecosystem.*
