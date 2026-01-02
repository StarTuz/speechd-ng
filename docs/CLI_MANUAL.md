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

Ask the AI "Brain" a question (requires Ollama to be running).

```bash
speechd-control think "What time is it in London?"
```

### Brain Management

Control the underlying AI model (Ollama).

```bash
# Check Brain status (online/offline, current model)
speechd-control brain

# Start/Stop the Ollama service
speechd-control brain start
speechd-control brain stop

# Switch to a different model
speechd-control brain use llama3:latest
speechd-control brain use mistral

# Pull a new model
speechd-control brain pull gemma:2b
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
