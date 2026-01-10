# Task: Phase 6 - Situational Awareness & Proactivity

- [x] Environmental Context Integration
  - [x] Research Window Manager (WM) IPC (X11/Wayland/Sway/i3)
  - [x] Implement `active_window` context adapter (X11)
  - [x] Implement `active_window` context adapter (Wayland - GNOME/KDE/Sway)
  - [x] Inject environmental metadata into Cortex prompt
- [/] Proactive Interaction Engine
  - [/] Implement event-based trigger system
  - [/] Implement event-based trigger system
    - [x] System Load Monitor (`sysinfo` integration) - *Implemented as Opt-in Service (CLI)*
    - [x] Timer/Countdown Logic
    - [x] Desktop Notifications (DBus Listener)
  - [x] Allow Cortex to initiate speech events
  - [x] Design rate-limiting and urgency levels for proactive speech
- [x] Predictive Synthesis Optimization
  - [x] Implement token-lookahead for engine preparation (Sentence Buffer)
  - [x] Reduce TTFB (Time to First Byte) via streaming pipeline
- [x] The Eye (Foundation)
  - [x] Prepare multimodal input channels in `Cortex` (`VisualQuery`)
  - [x] Implement `src/vision.rs` capture strategies
  - [x] Research lightweight local vision models (Moondream, etc.)
  - [x] Implement local vision engine (`TheEye` w/ Moondream 2)
  - [x] Integrate local vision into `Cortex` (`VisualQueryLocal`)
  - [x] Implement `DescribeScreen` D-Bus API
  - [x] Add `describe` command to `speechd-control` CLI

# Phase 7: Reliability & Testing

- [x] Comprehensive Test Harness
  - [x] Create `tests/integration_test.rs` to instantiate `SpeechService` with `MockAudioOutput`.
  - [x] Verify that `main.rs` and other components compile with the trait object changes.
  - [x] Verify via `cargo test`.
  - [x] create `docs/TESTING.md` and expand integration tests.
- [x] Dependency Maintenance
  - [x] Fix `clap` v4.5.53- [x] Implement Vision D-Bus API (`DescribeScreen`)
- [x] Implement Vision CLI subcommand (`describe`)
- [x] Test installation process and deploy binaries
- [x] Automate resource fetching (Vosk, Moondream)
- [x] Finalize Architecture & Handoff documentation
- [x] Brief Council on Moondream 2 configuration mismatch
- [x] Final Documentation Audit & Council Briefing
  - [x] Update `HANDOFF.md` with "The Eye" status
  - [x] Update `EXPERT_BRIEFING.md` with Moondream resolution
  - [x] Update `ARCHITECTURE_REVIEW.md` and `API_REFERENCE.md`
  - [x] Final Vision Verification (End-to-End Success)

## Phase 8: Pre-Brochure Verification (The Council's Mandate) [x]

- [x] Implement Concurrency Collision Test (Sprint)
- [x] Implement Adversarial Image Shape Test (Aris)
- [x] Implement Polkit Denial Audit (Bulwark)
- [x] Implement Chronicler Flooding Test (Viper)
- [x] Fix CI Linker Error (Forge) - *Implemented "Shotgun Strategy"*
- [x] Tune Stress Test Latency for Debug Builds
- [x] Final 'Status' CLI Discovery Polish (Elara)

# 🏁 Milestone: Version 1.0.0 Release [x]

- [x] Bump `Cargo.toml` version
- [x] Finalize `CHANGELOG.md`
- [x] Verify CI Pipeline GREEN
- [x] Create and Verify Git Tag `v1.0.0`
