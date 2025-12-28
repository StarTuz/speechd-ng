# Security Model & Policy

SpeechD-NG is designed to run as a central system service, potentially processing sensitive information (screen reading) and interacting with external services (LLMs). Security is therefore a foundational requirement, not an afterthought.

## Threat Model

### 1. Malicious Client Applications
*   **Threat**: A rogue application spams the speech server to annoy the user (DoS) or attempts to "listen" to the speech history to capture sensitive data (bank details, passwords read aloud).
*   **Mitigation**: 
    *   **Input**: Rate limiting on the D-Bus interface (planned).
    *   **Output**: The `Think()` API requires **Polkit Authentication** (hook implemented, enforcement pending). An app cannot read what *other* apps sent to the speech daemon without explicit user consent.

### 2. Remote Execution (The LLM Vector)
*   **Threat**: If the daemon talks to a remote LLM API, it could be vulnerable to Man-in-the-Middle (MITM) attacks or data leakage.
*   **Mitigation**:
    *   **Local First**: The Cortex prioritizes **Local LLMs** (Ollama on localhost).
    *   **Traffic Isolation**: The systemd sandbox restricts network access to `localhost` only via `IPAddressAllow=localhost` and `IPAddressDeny=any`.

### 3. Audio Injection
*   **Threat**: Hijacking the audio stream to play unauthorized sounds.
*   **Mitigation**: The `rodio` sink runs in an isolated thread. Input is passed via `std::process::Command::arg()` which is **immune to shell injection**.

### 4. Prompt Injection (LLM)
*   **Threat**: A malicious actor sends crafted text to manipulate the LLM's behavior.
*   **Mitigation**: User input is sanitized before being sent to Ollama (implemented). System prompts are separated from user content.

## Hardening Configuration

### Systemd Sandboxing (Implemented)
The service file (`systemd/speechd-ng.service`) includes comprehensive sandboxing:

```ini
[Service]
# === Filesystem Protection ===
ProtectSystem=strict          # /usr, /boot, /etc read-only
ProtectHome=read-only         # /home read-only
PrivateTmp=true               # Private /tmp namespace

# === Privilege Escalation Prevention ===
NoNewPrivileges=true          # Cannot gain new privileges
CapabilityBoundingSet=        # No capabilities allowed
AmbientCapabilities=          # No ambient capabilities

# === Kernel Protection ===
ProtectKernelTunables=true    # Cannot modify /proc/sys
ProtectKernelModules=true     # Cannot load kernel modules
ProtectKernelLogs=true        # Cannot read kernel logs
ProtectControlGroups=true     # Cannot modify cgroups
ProtectClock=true             # Cannot change system clock
ProtectHostname=true          # Cannot change hostname

# === Device Access ===
PrivateDevices=true           # No access to /dev (audio via socket)

# === Network Restrictions ===
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
IPAddressAllow=localhost      # Only localhost connections
IPAddressDeny=any             # Deny all other IPs

# === System Call Filtering ===
SystemCallArchitectures=native
SystemCallFilter=@system-service
SystemCallFilter=~@privileged @resources

# === Memory Protection ===
MemoryDenyWriteExecute=true   # W^X enforcement
LockPersonality=true          # Cannot change execution domain
RestrictRealtime=true         # Cannot acquire realtime scheduling
RestrictSUIDSGID=true         # Cannot create SUID/SGID files

# === Misc ===
UMask=0077                    # Restrictive file creation mask
```

### IPC Security (D-Bus)
Sensitive methods are protected by the `SecurityAgent`:
-   `Think()`: Requires `org.speech.service.think` permission (stub logs sender, enforcement planned).
-   `Speak()`: Currently unrestricted (public speech is generally allowed).

## Security Posture Summary

| Control | Status |
|---------|--------|
| Systemd Sandboxing | ✅ Implemented (20+ directives) |
| Network Isolation | ✅ Localhost only |
| Shell Injection Prevention | ✅ Safe `Command::arg()` usage |
| Polkit Permission Checks | ⚠️ Hook only (logs, does not deny) |
| LLM Prompt Sanitization | ✅ Implemented |
| Rate Limiting | ❌ Not yet implemented |

## Reporting Vulnerabilities
If you discover a security vulnerability, please do not open a public issue. Contact the maintainers directly via GitHub security advisories.
