# SpeechD-NG: Analysis & Evaluation

## 1. Project Goal

The goal of **SpeechD-NG** (Next Generation) was to create a modern replacement for the aging `speech-dispatcher` infrastructure on Linux.

* **Problem**: Legacy `speech-dispatcher` is written in C, uses an older architecture, can be prone to crashes/deadlocks, and lacks native integration with modern AI (LLMs) and Neural TTS.
* **Solution**: A Rust-based daemon that leverages:
  * **Memory Safety**: Rust ownership model prevents segfaults/buffer overflows.
  * **Async Concurrency**: `tokio` and `zbus` allow handling high-throughput D-Bus messages without blocking audio.
  * **Resiliency**: Worker threads with timeouts ensure that a hanging TTS engine never crashes the main service.
  * **Compatibility**: SSIP Shim allows legacy apps (Orca) to work unmodified.

## 2. Technical Capabilities

* **TTS Engine**:
  * **Pluggable Backend**: Currently supports `espeak-ng`. Easy validation for others (Piper, Coqui).
  * **Voice Management**: Enumerates and selects voices dynamically.
* **Audio Input (The "Ear")**:
  * **Microphone**: Uses `cpal` for cross-platform audio capture.
  * **STT**: Integrates with `vosk` (fast/offline) and `whisper` (accurate) CLIs.
* **Interfaces**:
  * **D-Bus**: `org.speech.Service` (Standard Linux IPC).
  * **SSIP (TCP 6560)**: Legacy text protocol for screen readers.
* **Security**:
  * **Polkit**: Critical actions (Listen, Think) require authorization.
  * **Systemd**: Uses `ProtectSystem=strict`, `PrivateTmp`, `NoNewPrivileges`.

## 3. Developer Guide

### Using via D-Bus (Recommended)

Developers should use the `org.speech.Service` D-Bus interface.

**Introspection XML**:

```xml
<node>
  <interface name="org.speech.Service">
    <method name="Speak">
      <arg name="text" type="s" direction="in"/>
    </method>
    <method name="SpeakVoice">
      <arg name="text" type="s" direction="in"/>
      <arg name="voice_id" type="s" direction="in"/>
    </method>
    <method name="ListVoices">
      <arg name="voices" type="a(ss)" direction="out"/>
    </method>
    <method name="Listen">
      <arg name="transcript" type="s" direction="out"/>
    </method>
    <method name="GetBrainStatus">
      <arg name="is_running" type="b" direction="out"/>
      <arg name="current_model" type="s" direction="out"/>
      <arg name="available_models" type="as" direction="out"/>
    </method>
    <method name="ManageBrain">
      <arg name="action" type="s" direction="in"/>
      <arg name="param" type="s" direction="in"/>
      <arg name="success" type="b" direction="out"/>
    </method>
  </interface>
</node>
```

**Python Example**:

```python
import dbus
bus = dbus.SessionBus()
proxy = bus.get_object('org.speech.Service', '/org/speech/Service')
interface = dbus.Interface(proxy, 'org.speech.Service')

# Speak
interface.Speak("Hello World")

# Listen
transcript = interface.Listen()
print(f"You said: {transcript}")
```

## 4. Evaluation vs. Ecosystem

| Feature | Speech-Dispatcher (Legacy) | SpeechD-NG (This Project) |
| :--- | :--- | :--- |
| **Language** | C / Python | **Rust** (Memory Safe) |
| **Architecture** | Forking / Multi-process | **Async Event Loop + Thread Pool** |
| **Resiliency** | Can hang if module hangs | **Timeout Mitigation** (kills hanging backends) |
| **Compatibility**| Native SSIP, Modules | **D-Bus Native**, SSIP Shim |
| **AI Integration** | No | **Yes** (Native Cortex/Ollama management) |
| **Input/STT** | No (Output only) | **Yes** (Mic + Vosk/Whisper/Wyoming) |
| **Config** | Complex `speechd.conf` | Modern TOML / Zero-conf defaults |

## 5. Summary of Major Milestones

1. **Neural TTS**: Implemented high-quality Piper integration (zero-config downloads).
2. **Robust AI**: Cortex provides 30s timeout buffers for local LLM cold-starts.
3. **Brain Management**: Stratus GUI can now start/stop and pull models via D-Bus.
4. **Multi-Channel**: Aviation-grade stereo panning and 5.1 surround support.
5. **Packaging**: Native `.deb`, `.rpm`, and `Flatpak` support (v0.7.2).
