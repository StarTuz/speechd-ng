# SpeechD-NG: The Next-Generation Linux Speech Daemon

**SpeechD-NG** is a modern, secure, and intelligent speech service designed for the Linux ecosystem. It aims to replace the aging `speech-dispatcher` with a window-manager agnostic, high-performance, and "AI-ready" architecture.

## 🚀 Mission
1.  **Window Manager Agnostic**: Works flawlessly on GNOME, KDE, Sway, Hyprland, and raw TTYs.
2.  **Service-Based**: Runs as a standard `systemd` service (User or System).
3.  **Secure by Design**: Uses D-Bus for IPC with strict isolation.
4.  **AI-Ready**: Built to integrate with local LLMs (like Ollama) for passive/active learning *without* blocking critical audio paths.
5.  **Fast**: Rust-based core with asynchronous audio processing.

## 🏗 Architecture

The system is composed of three main layers:

1.  **The Daemon (Core)**:
    -   **Technology**: Rust + `zbus`.
    -   **Role**: Extremely lightweight router. Accepts D-Bus calls, manages state, and enforces security.
    -   **Status**: ✅ Implemented.
2.  **The Audio Engine**:
    -   **Technology**: `rodio` + `espeak-ng` (thread-isolated).
    -   **Role**: Synthesizes text to speech in a dedicated thread. Ensures the main daemon *never* hangs, even if synthesis is slow.
    -   **Status**: ✅ Implemented.
3.  **The Cortex (Planned)**:
    -   **Technology**: Asynchronous Tokio tasks + HTTP (Ollama).
    -   **Role**: "The Brain". Listens to speech history and context to provide Active/Passive learning features (e.g., "Recall what I said 5 minutes ago").
    -   **Status**: 🚧 Planned (Phase 3).

## 🛠 Building & Installation

### Prerequisites
-   Rust (Stable)
-   `espeak-ng` (Runtime dependency for synthesis)
-   `libdbus-1-dev` (Usually pre-installed)

### Build
```bash
cargo build --release
```

### Installation (User Service)
1.  Copy the binary:
    ```bash
    cp target/release/speechserverdaemon ~/.local/bin/
    ```
2.  Install Systemd Unit:
    ```bash
    cp systemd/speechd-ng.service ~/.config/systemd/user/
    systemctl --user daemon-reload
    systemctl --user enable --now speechd-ng
    ```

## 📡 API Usage (D-Bus)

You can interact with the daemon using any D-Bus compliant tool or library.

**Bus**: Session Bus (`--user`)
**Service Name**: `org.speech.Service`
**Object Path**: `/org/speech/Service`
**Interface**: `org.speech.Service`

### Example: Command Line
```bash
# Speak (Body)
busctl --user call \
    org.speech.Service \
    /org/speech/Service \
    org.speech.Service \
    Speak s "Hello, this is the future of Linux speech."

# Think (Brain) - asks Ollama about the context
busctl --user call \
    org.speech.Service \
    /org/speech/Service \
    org.speech.Service \
    Think s "Summarize what you just said."
```

## 🗺 Roadmap

-   **Phase 1: Foundation** (✅ Completed) - Basic D-Bus Daemon.
-   **Phase 2: Audio Engine** (✅ Completed) - Thread-safe Audio Synthesis.
-   **Phase 3: The Cortex** (✅ Completed) - Ollama Integration & Context API.
-   **Phase 4: Security** - Polkit Integration & App permissions.
-   **Phase 5: Input & STT** - Microphone handling & Speech-to-Text streams.
