# specialized-control CLI Manual

`speechd-control` is the command-line interface for the SpeechD-NG daemon. It allows you to control speech synthesis, audio playback, AI integration, and voice training directly from the terminal.

## Basic Usage

```bash
speechd-control <COMMAND> [OPTIONS]
```

## Core Commands

### Status & Health

Check the daemon's status, version, and health.

```bash
# Show full status (version, AI state, patterns, etc.)
speechd-control status

# Simple ping to check if service is responsive (returns "pong")
speechd-control ping

# Show version number
speechd-control version
```

## Text-to-Speech (TTS)

### Speak Text

Speak text using the default or specified voice.

```bash
# Speak using default voice
speechd-control speak "Hello world"

# Speak using a specific voice
speechd-control speak "Hello world" --voice piper:en_GB-alba-medium

# Speak to a specific audio channel (left, right, center, rear-left, rear-right, lfe)
speechd-control speak "Left speaker test" --channel left
```

### Manage Voices

```bash
# List all installed voices
speechd-control voices

# List voices available for download
speechd-control voices --remote

# Download a specific voice
speechd-control download piper:en_US-lessac-medium
```

## Audio Playback

### Play Audio

Stream audio directly from a URL (supports file://, http://, https://).

```bash
# Play a file from URL
speechd-control play https://example.com/alert.mp3

# Play to a specific channel
speechd-control play https://example.com/alert.mp3 --channel right
```

### Control Playback

```bash
# Stop current playback immediately
speechd-control stop

# Get current volume
speechd-control volume

# Set volume (0.0 to 1.0)
speechd-control volume 0.8
```

### Audio Devices

```bash
# List available audio output sinks
speechd-control sinks
```

## AI Integration

### AI Queries

Ask the AI "Brain" a question.

```bash
speechd-control think "What time is it in London?"
```

### Brain Management

```bash
# Check Brain status (online/offline, current model)
speechd-control brain

# Start/Stop the Ollama service
speechd-control brain start
speechd-control brain stop

# Switch to a different Ollama model at runtime (no restart needed)
speechd-control brain use llama3:latest
speechd-control brain use mistral

# Pull a new Ollama model
speechd-control brain pull gemma:2b
```

## Config Reload

Apply changes from `~/.config/speechd-ng/Speech.toml` without restarting:

```bash
speechd-control reload
# or via systemd:
systemctl --user reload speechd-ng
```

Most settings take effect immediately. The only exception is `piper_binary`
(the path to the piper-tts binary), which requires a full restart.

---

## Switching Models

### TTS Voice (piper-tts)

```bash
# 1. Browse available voices
speechd-control voices --remote

# 2. Download the one you want
speechd-control download piper-tts:en_GB-alba-medium

# 3. Update your config
#    Edit ~/.config/speechd-ng/Speech.toml:
#      piper_model = "en_GB-alba-medium"

# 4. Reload — no restart needed
speechd-control reload

# Verify: speak something to confirm the new voice
speechd-control speak "Hello, this is the new voice"
```

Installed voices live in `~/.local/share/piper/models/`. If the configured
model is not found, the daemon falls back to the first available voice and
logs a warning to the journal.

### AI Backend: BitNet (default)

BitNet uses a GGUF model file in `~/bitnet/models/`. To switch models:

```bash
# 1. Download the new GGUF to ~/bitnet/models/
#    (must be a BitNet-compatible GGUF from a repo like larenspear/bitnet_b1_58-3B-GGUF)
cp /path/to/new-model.gguf ~/bitnet/models/

# 2. Edit the systemd unit to point at the new file
#    Edit ~/.config/systemd/user/bitnet.service, change -m in ExecStart:
#      ExecStart=... -m models/new-model.gguf --host 127.0.0.1 --port 8000 -ngl 0

# 3. Also update Speech.toml so the cortex knows the model name:
#      bitnet_model = "models/new-model"

# 4. Restart BitNet to load the new model
systemctl --user daemon-reload
systemctl --user restart bitnet

# 5. Reload speechd-ng config (picks up new bitnet_model name)
speechd-control reload
```

> **Note:** `-ngl 0` keeps all layers on CPU. BitNet 1-bit weights are
> optimised for CPU inference — do not remove this flag or llama-server will
> offload to GPU and consume several GB of VRAM for no throughput gain.

### AI Backend: Ollama

Ollama model switching is live — no restart required:

```bash
# Pull a model first (if not already downloaded)
speechd-control brain pull llama3

# Switch to it immediately
speechd-control brain use llama3

# Or edit Speech.toml and reload for a persistent change:
#   ollama_model = "llama3"
speechd-control reload
```

### Switching Between Backends (BitNet ↔ Ollama)

Edit `~/.config/speechd-ng/Speech.toml`:

```toml
ai_backend = "bitnet"   # use BitNet (default, CPU-only, no Ollama needed)
ai_backend = "ollama"   # use Ollama
ai_backend = "auto"     # try BitNet first, fall back to Ollama
```

Then reload:

```bash
speechd-control reload
```

## Voice Recognition & Training (VAD)

### Listen

Listen to microphone input and transcribe it (if STT is enabled).

```bash
speechd-control listen
```

### Training (Wake Word / Commands)

Train the system to recognize specific words or correct misheard phrases.

```bash
# Train a word (records for 3 seconds)
speechd-control train "computer" --duration 3

# Add a correction for frequent errors
# Format: speechd-control correct "what-it-heard" "what-you-meant"
speechd-control correct "hey jar fish" "hey jarvis"

# List all learned patterns/corrections
speechd-control patterns

# Undo the last added correction
speechd-control rollback
```
