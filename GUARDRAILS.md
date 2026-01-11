# Guardrails: SpeechD-NG

> Note for Agents: These are NON-NEGOTIABLE requirements. Code that violates these MUST NOT be merged.

## 1. AIAM: Agent Governance

### No-Touch Zones

- **FORBIDDEN:** Modifying or deleting system-level binaries (e.g. `/usr/bin/`) or configuration outside the workspace.
- **FORBIDDEN:** Deleting project source files based on heuristic assumptions without Explicit Verification (EV).
- **CAUTION:** Avoid manual edits to `Cargo.lock` unless resolving specific dependency conflicts.

### Action Risk Tiers

- **Tier 0 (Safe):** Read-only, linting, UI state.
- **Tier 1 (Normal):** Incremental code edits, new feature files, cleaning `dist/` artifacts.
- **Tier 2 (High-Risk):** Dependency changes, internal API overrides, schema migrations.
- **Tier 3 (Restricted):** DELETIONS of source code, binary changes, global system environment changes.

### Mandatory Verification (EV)

- **REQUIRED:** Before T2/T3 actions, agents MUST use `view_file` or `ls` to provide state proof to the user.
- **REQUIRED:** All T3 actions must be logged with a `justification` in the project audit log.

---

## 2. Universal Standards

### Input Validation

- **Confidence thresholding:** Reject ambiguous inputs.
- **Rate Limiting:** Prevent command flooding.

### Output Integrity

- **Entity Verification:** Ensure objects exist before acting.
- **High-Risk Confirmation:** Require "Say confirm to proceed" for dangerous commands.

### Error Handling

- **No Silent Failures:** Every error path must be logged or handled.
- **Trace-to-Fix:** Focus on execution flow, not environment assumptions.

### Audit Logging

- **Decision Tracking:** Log all commands with source, confidence, and action status.

---

## 3. Stability Doctrine (The Shiny Object Rules)

### 3.1 Core vs. Optional

- **Core Daemon**: Must remain compatible with LTS distributions (Debian Stable / Ubuntu LTS). "Boring is Good."
- **Optional Modules**: May use bleeding-edge deps (e.g. CUDA 12.6) ONLY IF they live in a separate binary (like `speechd-vision`) or feature flag.

### 3.2 Dependency Veto

- **Zero-Recall Rule**: If a dependency update requires users to manually install new system drivers (e.g. newer glibc/CUDA), it is **REJECTED** for the Core Daemon.
- **The "Marcus" Check**: Any PR bumping a `Cargo.lock` version must answer: "Does this break the build on the 3-year-old laptop?"

### 3.3 The One-Way Ratchet

- We do not regress. Once a privacy feature (like Mic Kill-Switch) is added, it cannot be "refactored out" for convenience.
