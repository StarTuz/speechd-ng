# SpeechD-NG v0.2.0 Release Notes

**Release Date:** 2025-12-28

This is a major milestone release marking the completion of all 14 development phases. SpeechD-NG is now **production-ready** with full CI validation and packaging support.

---

## 🎉 Highlights

- **All Phases Complete (1-14)**: From Foundation to Hardening & Packaging
- **Production-Hardened CI**: Robust offline-mode verification and D-Bus stability testing
- **Stable D-Bus Interface**: Explicit CamelCase method naming for guaranteed compatibility

---

## ✨ New Features (Phase 14)

### Diagnostics & Safety

- **`Ping()` Method**: Simple D-Bus connectivity check for monitoring and scripting
- **Improved Error Handling**: Graceful degradation when audio hardware or AI backends are unavailable
- **Rollback Support**: Undo accidental voice pattern learning with `RollbackLastCorrection()`

### CI/CD Hardening

- **Offline Resilience Tests**: Daemon survives network outages and missing audio devices
- **D-Bus Introspection**: Automatic interface validation in CI pipelines
- **Latency Benchmarking**: Performance test suite in `benchmarks/`

### Packaging

- **Debian Package**: `.deb` via `cargo-deb` with systemd integration
- **Flatpak Manifest**: Ready for Flathub submission
- **Systemd Service**: User-level service with 20+ security directives

---

## 📦 Installation

### From Source

```bash
cargo build --release
cp target/release/speechd-ng ~/.local/bin/
```

### Debian Package

```bash
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/speechd-ng_0.2.0_amd64.deb
```

---

## 🔧 D-Bus API Additions

| Method | Description |
|--------|-------------|
| `Ping()` | Returns "pong" - connectivity check |
| `GetStatus()` | Diagnostic summary (AI enabled, STT backend, pattern count) |

---

## 📋 Full Changelog

### Added

- `Ping()` D-Bus method for diagnostics
- Offline resilience CI workflow
- D-Bus introspection in CI for interface validation
- Latency benchmark suite (`benchmarks/latency_test.py`)

### Changed

- Downgraded `zbus` to 4.4.0 for macro stability
- Explicit CamelCase D-Bus method naming (no more auto-conversion issues)
- Improved error messages for headless environments

### Fixed

- D-Bus method discovery issues in headless CI
- Audio stream panics when no hardware present
- Configuration loading panics for missing optional fields

---

## 🙏 Acknowledgments

Thank you for using SpeechD-NG! This release represents the culmination of 14 development phases and establishes a solid foundation for future enhancements.

---

*SpeechD-NG: Speak freely.*
