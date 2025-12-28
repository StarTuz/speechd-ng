# Security Model & Policy

SpeechD-NG is designed to run as a central system service, potentially processing sensitive information (screen reading) and interacting with external services (LLMs). Security is therefore a foundational requirement, not an afterthought.

## Threat Model

### 1. Malicious Client Applications
*   **Threat**: A rogue application spams the speech server to annoy the user (DoS) or attempts to "listen" to the speech history to capture sensitive data (bank details, passwords read aloud).
*   **Mitigation**: 
    *   **Input**: Rate limiting on the D-Bus interface.
    *   **Output**: The "History" or "Context" API will require **Polkit Authentication**. An app cannot read what *other* apps sent to the speech daemon without explicit user consent (via a system password or prompt).

### 2. Remote Execution (The LLM Vector)
*   **Threat**: If the daemon talks to a remote LLM API, it could be vulnerable to Man-in-the-Middle (MITM) attacks or data leakage.
*   **Mitigation**:
    *   **Local First**: The Cortex is designed to prioritize **Local LLMs** (Ollama on localhost). This keeps data on the device.
    *   **Traffic Isolation**: If a remote API *is* used, it should be done over HTTPS with strictly validated certificates. The daemon should run in a systemd sandbox that only allows network access to specific IPs (or localhost).

### 3. Audio Injection
*   **Threat**: Hijacking the audio stream to play unauthorized sounds.
*   **Mitigation**: The `rodio` sink runs in a thread that isolates it from the main input loop. Input sanitization is performed effectively by `espeak-ng` (text-only), but we must ensure no shell-injection vulnerabilities exist when invoking the synthesizer command. (Current implementation uses `std::process::Command` with argument separating, which is safe from standard shell injection).

## Hardening Configuration

### Systemd Sandboxing
We employ strict systemd directives to minimize the blast radius if the daemon is compromised.

```ini
[Service]
# Service cannot write to most of the FS
ProtectSystem=strict
# Service cannot access /home (except its own config if needed)
ProtectHome=read-only
# No direct access to hardware devices (audio handled via PipeWire socket)
PrivateDevices=true
# Prevent escalating privileges
NoNewPrivileges=true
```

### IPC Security (D-Bus)
Calls to sensitive methods (like `GetSpeechHistory` or `ConfigureVoice`) will be protected by XML policy definitions in `/usr/share/dbus-1/system-services/`.

## Reporting Vulnerabilities
If you discover a security vulnerability, please do not open a public issue. Contact the maintainers directly.
