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
