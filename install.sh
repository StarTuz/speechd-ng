#!/bin/bash
set -e

# SpeechD-NG Installer

CONFIG_DIR="$HOME/.config/speechd-ng"
CONFIG_FILE="$CONFIG_DIR/Speech.toml"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "========================================"
echo "   SpeechD-NG Installer (v0.7.2)"
echo "========================================"

# Check if we're in the project directory with built binaries
if [ -f "target/release/speechd-ng" ]; then
    INSTALL_MODE="source"
    echo "[*] Detected source installation mode"
else
    INSTALL_MODE="package"
    echo "[*] Detected package installation mode"
fi

if [ "$INSTALL_MODE" == "source" ]; then
    # Source Installation
    echo "[*] Installing from source..."
    
    mkdir -p "$BIN_DIR"
    mkdir -p "$SYSTEMD_DIR"
    
    # Copy binaries
    echo "    Installing speechd-ng..."
    cp target/release/speechd-ng "$BIN_DIR/"
    
    if [ -f "target/release/speechd-control" ]; then
        echo "    Installing speechd-control..."
        cp target/release/speechd-control "$BIN_DIR/"
    fi
    
    # Copy Python bridges
    if [ -f "src/wakeword_bridge.py" ]; then
     # Bridges are now internal to the Rust binary
        : # No Python bridges to install
    fi
    
    # Copy systemd service
    echo "    Installing systemd service..."
    cp systemd/speechd-ng.service "$SYSTEMD_DIR/"
    
else
    # Package Installation
    echo "[*] Detecting Distribution..."
    if [ -f /etc/debian_version ]; then
        DISTRO="debian"
        echo "    Detected: Debian/Ubuntu based"
    elif [ -f /etc/redhat-release ]; then
        DISTRO="redhat"
        echo "    Detected: Fedora/RHEL based"
    else
        echo "    ERROR: Unknown distribution."
        echo "    Please build from source: cargo build --release && ./install.sh"
        exit 1
    fi

    echo "[*] Locating Package..."
    if [ "$DISTRO" == "debian" ]; then
        # Try new name first, fall back to old name
        PKG=$(find dist -name "speechd-ng_*_amd64.deb" 2>/dev/null | sort -V | tail -n1)
        if [ -z "$PKG" ]; then
            PKG=$(find dist -name "speechserverdaemon_*_amd64.deb" 2>/dev/null | sort -V | tail -n1)
        fi
        if [ -z "$PKG" ]; then
            echo "Error: No .deb package found in dist/"
            echo "Build from source first: cargo build --release"
            exit 1
        fi
        INSTALL_CMD="sudo apt-get install -y ./$PKG"
        sudo apt-get update
        sudo apt-get install -y build-essential curl pkg-config libasound2-dev libdbus-1-dev espeak-ng ffmpeg libvosk-dev
    elif [ "$DISTRO" == "redhat" ]; then
        PKG=$(find dist -name "speechd-ng-*.x86_64.rpm" 2>/dev/null | sort -V | tail -n1)
        if [ -z "$PKG" ]; then
            PKG=$(find dist -name "speechserverdaemon-*.x86_64.rpm" 2>/dev/null | sort -V | tail -n1)
        fi
        if [ -z "$PKG" ]; then
            echo "Error: No .rpm package found in dist/"
            echo "Build from source first: cargo build --release"
            exit 1
        fi
        INSTALL_CMD="sudo dnf install -y ./$PKG"
        sudo dnf install -y alsa-lib-devel dbus-devel espeak-ng ffmpeg-devel libvosk-devel
    fi

    echo "    Found: $PKG"
    read -p "    Install this package? [Y/n] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ -n $REPLY ]]; then
        echo "Aborting installation."
        exit 1
    fi

    echo "[*] Installing Package..."
    $INSTALL_CMD
fi

# Configuration Wizard
echo "[*] Configuration Wizard"
mkdir -p "$CONFIG_DIR"
chmod 700 "$CONFIG_DIR"

# Skip config wizard if config already exists
if [ -f "$CONFIG_FILE" ]; then
    echo "    Configuration already exists at $CONFIG_FILE"
    read -p "    Overwrite? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "    Keeping existing configuration."
        SKIP_CONFIG=true
    fi
fi

if [ "$SKIP_CONFIG" != "true" ]; then
    # Defaults
    WAKE_WORD="wendy"
    ENABLE_AI="false"
    STT_BACKEND="vosk"

    # Wake Word
    echo
    echo "--- Wake Word Selection ---"
    echo "1) Wendy (default)"
    echo "2) Alexa"
    echo "3) Computer"
    echo "4) Custom (enter manually)"
    read -p "Select [1-4]: " ww_choice
    case $ww_choice in
        2) WAKE_WORD="alexa" ;;
        3) WAKE_WORD="computer" ;;
        4) read -p "Enter custom wake word (snake_case): " WAKE_WORD ;;
        *) WAKE_WORD="wendy" ;;
    esac

    # AI
    echo
    echo "--- AI Integration ---"
    read -p "Enable Ollama integration? (requires local Ollama) [y/N]: " ai_choice
    if [[ $ai_choice =~ ^[Yy]$ ]]; then
        ENABLE_AI="true"
    fi

    # STT
    echo
    echo "--- Speech to Text ---"
    echo "1) Vosk (Local, Embedded)"
    echo "2) Wyoming (Remote/Home Assistant)"
    read -p "Select [1-2]: " stt_choice
    if [ "$stt_choice" == "2" ]; then
        STT_BACKEND="wyoming"
    fi

    # Write Config (Flat structure to match src/config_loader.rs)
    echo "[*] Writing configuration to $CONFIG_FILE..."
    cat > "$CONFIG_FILE" <<EOF
# SpeechD-NG Configuration
# Generated by install.sh on $(date)

# AI & Context
ollama_url = "http://localhost:11434"
ollama_model = "llama3"
enable_ai = $ENABLE_AI
passive_confidence_threshold = 0.1
memory_size = 50

# Audio & Performance
enable_audio = true
playback_volume = 1.0
playback_timeout_secs = 30
max_audio_size_mb = 50
global_audio_buffer_limit_mb = 200

# TTS Settings
tts_backend = "piper"
piper_model = "en_US-lessac-medium"
piper_binary = "piper"

# STT & Wake Word
stt_backend = "$STT_BACKEND"
wake_word = "$WAKE_WORD"
enable_wake_word = true

# VAD Settings
vad_speech_threshold = 500
vad_silence_threshold = 400
vad_silence_duration_ms = 1500
vad_max_duration_ms = 15000

# Wyoming Native Link
wyoming_host = "127.0.0.1"
wyoming_port = 10301
wyoming_auto_start = true
wyoming_device = "cpu"
wyoming_model = "tiny"

# Whisper Native Link
whisper_model_path = "$HOME/.cache/whisper/ggml-tiny.en.bin"
whisper_language = "en"

# Rate Limiting
rate_limit_tts = 30
rate_limit_ai = 10
rate_limit_audio = 20
rate_limit_listen = 30
EOF
fi

# Ensure required directories and model cache exist
echo "[*] Ensuring required directories and model cache exist..."
mkdir -p "$HOME/.local/share/piper/models"
mkdir -p "$HOME/.local/share/speechd-ng"
mkdir -p "$HOME/.cache/vosk"
mkdir -p "$HOME/.cache/whisper"

if [ "$STT_BACKEND" == "vosk" ] && [ ! -d "$HOME/.cache/vosk/vosk-model-small-en-us-0.15" ]; then
    echo "[*] Vosk model not found. Downloading..."
    mkdir -p "$HOME/.cache/vosk"
    cd "$HOME/.cache/vosk"
    if command -v curl >/dev/null 2>&1; then
        curl -L https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip -o model.zip
    else
        wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip -o model.zip
    fi
    echo "[*] Extracting model..."
    unzip model.zip
    rm model.zip
    cd - >/dev/null
fi

# Enable service
echo "[*] Enabling Service..."
systemctl --user daemon-reload
systemctl --user enable --now speechd-ng

echo "========================================"
echo "   Installation Complete!"
echo "========================================"
echo
echo "Binaries installed to: $BIN_DIR"
echo "Configuration saved to: $CONFIG_FILE"
echo
echo "Commands available:"
echo "  speechd-ng       - The daemon (runs as systemd service)"
echo "  speechd-control  - CLI control utility"
echo
echo "To restart the service after changes, run:"
echo "   systemctl --user restart speechd-ng"
echo
echo "Enjoy!"
