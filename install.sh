#!/bin/bash
set -e

# SpeechD-NG Installer v1.0.0
# Core daemon + optional Vision service

CONFIG_DIR="$HOME/.config/speechd-ng"
CONFIG_FILE="$CONFIG_DIR/Speech.toml"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "========================================"
echo "   SpeechD-NG Installer (v1.0.0)"
echo "========================================"

# Check if we're in the source directory
if [ ! -f "Cargo.toml" ]; then
    echo "ERROR: Run this script from the speechd-ng source directory"
    exit 1
fi

echo "[*] Detected source directory"

# Stop services before installation
if systemctl --user is-active --quiet speechd-ng 2>/dev/null; then
    echo "[*] Stopping speechd-ng service..."
    systemctl --user stop speechd-ng
fi
if systemctl --user is-active --quiet speechd-vision 2>/dev/null; then
    echo "[*] Stopping speechd-vision service..."
    systemctl --user stop speechd-vision
fi

# ============================================================================
# Core Installation
# ============================================================================
echo ""
echo "--- Core Daemon Installation ---"

NEED_BUILD=false
if [ ! -f "target/release/speechd-ng" ]; then
    NEED_BUILD=true
elif [ "$1" == "--rebuild" ]; then
    NEED_BUILD=true
else
    echo "[*] Existing build found"
    read -p "    Rebuild core daemon? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        NEED_BUILD=true
    fi
fi

if [ "$NEED_BUILD" = true ]; then
    echo "[*] Building core daemon (no ML dependencies)..."
    cargo build --release --bin speechd-ng --bin speechd-control
    if [ $? -ne 0 ]; then
        echo "    ERROR: Build failed!"
        exit 1
    fi
fi

echo "[*] Installing core binaries..."
mkdir -p "$BIN_DIR"
mkdir -p "$SYSTEMD_DIR"

cp target/release/speechd-ng "$BIN_DIR/"
cp target/release/speechd-control "$BIN_DIR/"
cp systemd/speechd-ng.service "$SYSTEMD_DIR/"

echo "[*] Core daemon installed successfully"

# ============================================================================
# Optional Vision Service
# ============================================================================
echo ""
echo "--- Vision Service (Optional) ---"
echo ""
echo "The Vision service (The Eye) provides screen description using AI."
echo "It requires ~2GB disk space for the Moondream 2 model."
echo ""
echo "Performance:"
echo "  - With CUDA (11.x-12.6): 1-3 seconds per image"
echo "  - Without CUDA: 30-60+ seconds per image (not recommended)"
echo ""

read -p "Install Vision service? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    INSTALL_VISION=true
else
    INSTALL_VISION=false
    echo "[*] Skipping Vision service"
fi

if [ "$INSTALL_VISION" = true ]; then
    # Check CUDA availability
    CUDA_FLAGS=""
    if command -v nvidia-smi &>/dev/null && nvidia-smi &>/dev/null; then
        GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -n1)
        echo "[*] NVIDIA GPU detected: $GPU_NAME"

        if command -v nvcc &>/dev/null; then
            CUDA_VERSION=$(nvcc --version | grep "release" | sed 's/.*release \([0-9]*\.[0-9]*\).*/\1/')
            CUDA_MAJOR=$(echo "$CUDA_VERSION" | cut -d. -f1)

            if [ "$CUDA_MAJOR" -ge 11 ] && [ "$CUDA_MAJOR" -le 12 ]; then
                echo "[*] CUDA $CUDA_VERSION detected (supported)"
                CUDA_FLAGS="--features cuda"
            else
                echo "[!] CUDA $CUDA_VERSION is not supported (need 11.x-12.6)"
                echo ""
                echo "Options:"
                echo "  1) Install CUDA 12.x via NVIDIA runfile (recommended, ~5 min)"
                echo "     https://developer.nvidia.com/cuda-12-6-0-download-archive"
                echo "     Select: Linux > x86_64 > Your distro > runfile (local)"
                echo "     WARNING: Do NOT use AUR cuda packages - they compile GCC from source (2-4+ hours)"
                echo "  2) Continue with CPU (30-60+ seconds per image, not recommended)"
                echo "  3) Skip Vision service (recommended if you don't need screen description)"
                echo ""
                read -p "Choose [1/2/3]: " cuda_choice
                case $cuda_choice in
                    1)
                        echo "Please install CUDA 12.x and run: ./install.sh"
                        echo "Or run: ./install-vision.sh after installing CUDA"
                        INSTALL_VISION=false
                        ;;
                    2)
                        echo "[!] Building for CPU (this will be very slow)"
                        ;;
                    *)
                        INSTALL_VISION=false
                        ;;
                esac
            fi
        else
            echo "[!] CUDA toolkit (nvcc) not found"
            echo ""
            echo "Options:"
            echo "  1) Install CUDA toolkit first"
            echo "     - Ubuntu/Debian: sudo apt install nvidia-cuda-toolkit"
            echo "     - Fedora: sudo dnf install cuda-toolkit-12-6"
            echo "     - Arch: Use NVIDIA runfile (pacman cuda may be 13.x, AUR compiles GCC for hours)"
            echo "       https://developer.nvidia.com/cuda-12-6-0-download-archive"
            echo "  2) Continue with CPU (30-60+ seconds per image, not recommended)"
            echo "  3) Skip Vision service (recommended if you don't need screen description)"
            echo ""
            read -p "Choose [1/2/3]: " cuda_choice
            case $cuda_choice in
                1)
                    echo "Please install CUDA toolkit and run: ./install.sh"
                    INSTALL_VISION=false
                    ;;
                2)
                    echo "[!] Building for CPU (this will be very slow)"
                    ;;
                *)
                    INSTALL_VISION=false
                    ;;
            esac
        fi
    else
        echo "[!] No NVIDIA GPU detected"
        echo "    Vision service will be extremely slow on CPU (30-60+ seconds)"
        echo ""
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            INSTALL_VISION=false
        fi
    fi
fi

if [ "$INSTALL_VISION" = true ]; then
    echo "[*] Building Vision service..."
    if [ -n "$CUDA_FLAGS" ]; then
        echo "    With CUDA support"
        cargo build --release --bin speechd-vision $CUDA_FLAGS
    else
        echo "    CPU-only (will be slow)"
        cargo build --release --bin speechd-vision --features vision
    fi

    if [ $? -eq 0 ]; then
        echo "[*] Installing Vision service..."
        cp target/release/speechd-vision "$BIN_DIR/"
        cp systemd/speechd-vision.service "$SYSTEMD_DIR/"
        mkdir -p "$HOME/.cache/huggingface"
        mkdir -p "$HOME/.cache/speechd-vision"
        VISION_INSTALLED=true
        echo "[*] Vision service installed successfully"
    else
        echo "[!] Vision build failed, skipping"
        VISION_INSTALLED=false
    fi
else
    VISION_INSTALLED=false
fi

# ============================================================================
# Configuration
# ============================================================================
echo ""
echo "--- Configuration ---"
mkdir -p "$CONFIG_DIR"
chmod 700 "$CONFIG_DIR"

if [ -f "$CONFIG_FILE" ]; then
    echo "[*] Configuration already exists at $CONFIG_FILE"
    read -p "    Overwrite? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "    Keeping existing configuration."
        SKIP_CONFIG=true
    fi
fi

if [ "$SKIP_CONFIG" != "true" ]; then
    WAKE_WORD="wendy"
    ENABLE_AI="false"
    STT_BACKEND="vosk"

    echo ""
    echo "--- Wake Word Selection ---"
    echo "1) Wendy (default)"
    echo "2) Alexa"
    echo "3) Computer"
    echo "4) Custom"
    read -p "Select [1-4]: " ww_choice
    case $ww_choice in
        2) WAKE_WORD="alexa" ;;
        3) WAKE_WORD="computer" ;;
        4) read -p "Enter wake word: " WAKE_WORD ;;
        *) WAKE_WORD="wendy" ;;
    esac

    echo ""
    read -p "Enable Ollama AI integration? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        ENABLE_AI="true"
    fi

    echo ""
    echo "--- Speech to Text ---"
    echo "1) Vosk (Local)"
    echo "2) Wyoming (Remote)"
    read -p "Select [1-2]: " stt_choice
    if [ "$stt_choice" == "2" ]; then
        STT_BACKEND="wyoming"
    fi

    echo "[*] Writing configuration..."
    cat > "$CONFIG_FILE" <<EOF
# SpeechD-NG Configuration
# Generated on $(date)

# AI & Context
ollama_url = "http://localhost:11434"
ollama_model = "llama3"
enable_ai = $ENABLE_AI
passive_confidence_threshold = 0.1
memory_size = 50
enable_rag = false

# Audio
enable_audio = true
playback_volume = 1.0
playback_timeout_secs = 30
max_audio_size_mb = 50
global_audio_buffer_limit_mb = 200

# TTS
tts_backend = "piper"
piper_model = "en_US-lessac-medium"
piper_binary = "piper"

# STT & Wake Word
stt_backend = "$STT_BACKEND"
wake_word = "$WAKE_WORD"
enable_wake_word = false
enable_microphone = false
vosk_model_path = "/usr/share/vosk/model"

# VAD
vad_speech_threshold = 500
vad_silence_threshold = 400
vad_silence_duration_ms = 1500
vad_max_duration_ms = 15000

# Wyoming
wyoming_host = "127.0.0.1"
wyoming_port = 10301
wyoming_auto_start = true
wyoming_device = "cpu"
wyoming_model = "tiny"

# Whisper
whisper_model_path = "$HOME/.cache/whisper/ggml-tiny.en.bin"
whisper_language = "en"

# Rate Limiting
rate_limit_tts = 30
rate_limit_ai = 10
rate_limit_audio = 20
rate_limit_listen = 30

# Governance
system_prompt = "You are the SpeechD-NG Governance Brain. Your priority is absolute accuracy and hardware-awareness. 1. If you are provided with vision data (images), be highly skeptical. Small models like Moondream are prone to hallucination. 2. If vision data looks low-quality, ambiguous, or if you are unsure, state: 'Analysis inconclusive'. 3. Do not guess terminal errors or complex code from low-resolution vision data. 4. Always prioritize user safety and security over helpfulness. 5. If the user asks about system issues, admit when information is missing or when fallback (like CPU-only inference) might be degrading the experience."
EOF
fi

# ============================================================================
# Finalize
# ============================================================================
echo ""
echo "[*] Creating directories..."
mkdir -p "$HOME/.local/share/piper/models"
mkdir -p "$HOME/.local/share/speechd-ng"
mkdir -p "$HOME/.cache/vosk"

echo "[*] Enabling services..."
systemctl --user daemon-reload
systemctl --user enable --now speechd-ng

if [ "$VISION_INSTALLED" = true ]; then
    systemctl --user enable speechd-vision
    echo "    Vision service enabled (start with: systemctl --user start speechd-vision)"
fi

echo ""
echo "========================================"
echo "   Installation Complete!"
echo "========================================"
echo ""
echo "Installed:"
echo "  - speechd-ng (core daemon) - RUNNING"
echo "  - speechd-control (CLI)"
if [ "$VISION_INSTALLED" = true ]; then
echo "  - speechd-vision (The Eye) - ENABLED"
fi
echo ""
echo "Commands:"
echo "  speechd-control speak 'Hello world'"
echo "  speechd-control listen"
echo "  speechd-control think 'What is the meaning of life?'"
if [ "$VISION_INSTALLED" = true ]; then
echo "  speechd-control describe 'What do you see?'"
fi
echo ""
echo "Services:"
echo "  systemctl --user status speechd-ng"
echo "  systemctl --user restart speechd-ng"
if [ "$VISION_INSTALLED" = true ]; then
echo "  systemctl --user start speechd-vision"
fi
echo ""
