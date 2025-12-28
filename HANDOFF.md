# Project Handoff: SpeechD-NG

## Current Context
We are building **SpeechD-NG**, a modern replacement for Linux command-line/desktop speech services. The project is written in **Rust** to ensure memory safety, speed, and concurrency.

## Status: Phase 3 Completed (Brain & Body)
We has successfully implemented the **Audio Engine** (Body) and the **Cortex** (Brain).

### 1. Functional Features
-   **D-Bus Service**: The daemon claims `org.speech.Service` on the user bus.
-   **Async Core**: Uses `tokio` and `zbus` for non-blocking I/O.
-   **Audio Pipeline**: A dedicated thread handles `espeak-ng` generation + `rodio` playback.
-   **The Cortex**: A separate async module that:
    -   Observes all spoken text and stores it in a short-term memory buffer (default: 50 items).
    -   Connects to a local **Ollama** instance (`http://localhost:11434`) to answer questions about the speech context.
    -   Exposes a `Think(query)` method via D-Bus.

### 2. Key Technical Decisions
-   **Rodio v0.17.3**: Pinned for stability with the threaded actor model.
-   **Dual-Actor Architecture**: 
    -   `AudioEngine` (Sync Thread): Handles blocking audio operations.
    -   `Cortex` (Async Task): Handles HTTP requests and state management.
    -   `SpeechService` (Main): Dispatches to both in parallel.

## File Structure
-   `src/main.rs`: Entry point. Dispatches D-Bus calls.
-   `src/engine.rs`: Audio synthesis actor.
-   `src/cortex.rs`: Intelligence & Memory actor.
-   `systemd/`: Service files.

## Immediate Next Steps (Phase 4)
The next major milestone is **Security & Polish**.

1.  **Polkit Integration**: 
    -   The `Think` method exposes sensitive history. We *must* gate this behind a Polkit action (e.g., `org.speech.service.query-memory`) so random scripts can't snoop.
2.  **Configuration**:
    -   Allow users to configure the Ollama Model (currently hardcoded to `llama3`) and Memory Size.
    -   Voice selection for `espeak-ng`.

## Future Considerations
-   **Polkit**: We need to ensure arbitrary apps can't just spam the speech server or listen to history without permission.
-   **Voices**: Currently hardcoded to `espeak-ng` default. We need a way to pass voice parameters via D-Bus.
