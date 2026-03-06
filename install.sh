#!/bin/bash
set -e

# SpeechD-NG Installer v1.1.0
# Core daemon + optional Vision service

CONFIG_DIR="$HOME/.config/speechd-ng"
CONFIG_FILE="$CONFIG_DIR/Speech.toml"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "========================================"
echo "   SpeechD-NG Installer (v1.1.0)"
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
    echo "2) Computer"
    echo "3) Custom"
    read -p "Select [1-3]: " ww_choice
    case $ww_choice in
        2) WAKE_WORD="computer" ;;
        3) read -p "Enter wake word: " WAKE_WORD ;;
        *) WAKE_WORD="wendy" ;;
    esac

    echo ""
    echo "--- AI Brain Selection ---"
    echo "1) BitNet (Local, auto-start - Default)"
    echo "2) Ollama (Native REST)"
    echo "3) Auto (BitNet primary, Ollama fallback)"
    read -p "Select AI Backend [1-3]: " ai_choice
    case $ai_choice in
        2)
            AI_BACKEND="ollama"
            BITNET_AUTO_START="false"
            read -p "Enable Ollama AI integration? [y/N]: " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                ENABLE_AI="true"
            fi
            ;;
        3)
            AI_BACKEND="auto"
            BITNET_AUTO_START="true"
            ENABLE_AI="true"
            ;;
        *)
            AI_BACKEND="bitnet"
            BITNET_AUTO_START="true"
            ENABLE_AI="true"
            ;;
    esac

    # BitNet setup — fully automatic, no user input required
    BITNET_WORKDIR="$HOME/bitnet"
    BITNET_BIN="llama-server"
    BITNET_MODEL_REL="models/bitnet_b1_58-3B-Q4_K_M.gguf"

    install_bitnet() {
        mkdir -p "$BITNET_WORKDIR/models"

        # --- llama-server ---
        if command -v llama-server &>/dev/null; then
            BITNET_BIN=$(command -v llama-server)
            echo "[*] llama-server found: $BITNET_BIN"
        else
            echo "[*] Installing llama-server..."

            # Try package manager first (silent — don't spam output on failure)
            local pkg_ok=false
            if command -v pacman &>/dev/null; then
                sudo pacman -S --noconfirm llama-cpp &>/dev/null && pkg_ok=true
            elif command -v apt-get &>/dev/null; then
                sudo apt-get install -y llama-cpp &>/dev/null && pkg_ok=true
            elif command -v dnf &>/dev/null; then
                sudo dnf install -y llama-cpp &>/dev/null && pkg_ok=true
            fi

            if $pkg_ok && command -v llama-server &>/dev/null; then
                BITNET_BIN=$(command -v llama-server)
                echo "[*] llama-server installed via package manager"
            else
                # Auto-detect GPU → pick Vulkan or CPU build
                local variant="cpu-avx2"
                if nvidia-smi &>/dev/null 2>&1 || \
                   (command -v vulkaninfo &>/dev/null && vulkaninfo &>/dev/null 2>&1); then
                    variant="vulkan"
                    echo "[*] GPU detected — downloading Vulkan build"
                else
                    echo "[*] No GPU detected — downloading CPU build"
                fi

                local tag
                tag=$(curl -sf "https://api.github.com/repos/ggml-org/llama.cpp/releases/latest" \
                    | grep '"tag_name"' | cut -d'"' -f4)
                [ -z "$tag" ] && { echo "[!] Cannot reach GitHub."; return 1; }
                echo "[*] Downloading llama-server $tag..."

                local url="https://github.com/ggml-org/llama.cpp/releases/download/${tag}/llama-${tag}-bin-ubuntu-${variant}-x64.tar.gz"
                local tmp
                tmp=$(mktemp -d)
                if ! curl -L --progress-bar -o "$tmp/llama.tar.gz" "$url"; then
                    echo "[!] Download failed."; rm -rf "$tmp"; return 1
                fi
                tar -xzf "$tmp/llama.tar.gz" -C "$tmp"

                local found_bin
                found_bin=$(find "$tmp" -name "llama-server" -type f | head -n1)
                [ -z "$found_bin" ] && { echo "[!] llama-server not found in archive."; rm -rf "$tmp"; return 1; }

                cp "$found_bin" "$BIN_DIR/llama-server"
                chmod +x "$BIN_DIR/llama-server"
                # Co-locate .so files — ggml dlopen()s backends from the binary's directory
                find "$tmp" -name "*.so*" -type f -exec cp {} "$BIN_DIR/" \;
                rm -rf "$tmp"
                BITNET_BIN="$BIN_DIR/llama-server"

                # Verify no missing libs
                local missing
                missing=$(ldd "$BITNET_BIN" 2>/dev/null | grep "not found" || true)
                if [ -n "$missing" ]; then
                    echo "[!] llama-server has unresolved libraries:"
                    echo "$missing"
                    return 1
                fi
                echo "[*] llama-server installed: $BITNET_BIN"
            fi
        fi

        # --- BitNet model ---
        local model_abs="$BITNET_WORKDIR/$BITNET_MODEL_REL"
        if [ -f "$model_abs" ]; then
            echo "[*] Model already present."
            return 0
        fi

        echo "[*] Downloading BitNet model (~2.3 GB)..."
        # larenspear's repo — verified compatible with llama-server b8209+
        local hf_repo="larenspear/bitnet_b1_58-3B-GGUF"
        local filename
        filename=$(curl -sf "https://huggingface.co/api/models/${hf_repo}" | \
            python3 -c '
import sys, json
data = json.load(sys.stdin)
files = [s["rfilename"] for s in data.get("siblings", []) if s["rfilename"].endswith(".gguf")]
for q in ["q4_k_m", "q4_k_s", "q4_0"]:
    for f in files:
        if q in f.lower():
            print(f); exit(0)
' 2>/dev/null) || true

        [ -z "$filename" ] && { echo "[!] Could not find a compatible model."; return 1; }

        local model_url="https://huggingface.co/${hf_repo}/resolve/main/${filename}"
        BITNET_MODEL_REL="models/$(basename "$filename")"
        model_abs="$BITNET_WORKDIR/$BITNET_MODEL_REL"
        echo "[*] Fetching $(basename "$filename")..."
        if ! curl -L --progress-bar -o "$model_abs" "$model_url"; then
            rm -f "$model_abs"
            echo "[!] Model download failed."
            return 1
        fi
        echo "[*] Model ready."
    }

    if [[ "$AI_BACKEND" == "bitnet" || "$AI_BACKEND" == "auto" ]]; then
        echo ""
        echo "--- BitNet Setup ---"
        install_bitnet || echo "[!] BitNet setup failed — AI features will use Ollama fallback."
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
ai_backend = "$AI_BACKEND"
ollama_url = "http://localhost:11434"
ollama_model = "llama3"
bitnet_url = "http://localhost:8000"
bitnet_model = "models/bitnet_b1_58-3B"
bitnet_auto_start = $BITNET_AUTO_START
enable_ai = $ENABLE_AI
enable_vision = false
passive_confidence_threshold = 0.1
memory_size = 50
enable_rag = false
rag_top_k = 3

# Audio
enable_audio = true
playback_volume = 1.0
playback_timeout_secs = 30
max_audio_size_mb = 50
global_audio_buffer_limit_mb = 200

# TTS
tts_backend = "piper-tts"
piper_model = "$(find "$HOME/.local/share/piper/models" -name "*.onnx" 2>/dev/null | head -1 | xargs basename 2>/dev/null | sed 's/\.onnx$//' || echo 'en_US-lessac-medium')"
piper_binary = "piper-tts"

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

# Read AI_BACKEND from existing config if it was not set interactively
if [ -z "${AI_BACKEND:-}" ] && [ -f "$CONFIG_FILE" ]; then
    AI_BACKEND=$(grep '^ai_backend' "$CONFIG_FILE" | cut -d'"' -f2)
fi

# Only (re)write the BitNet service file when we did interactive setup
if [ "${SKIP_CONFIG:-false}" != "true" ] && [[ "${AI_BACKEND:-}" == "bitnet" || "${AI_BACKEND:-}" == "auto" ]]; then
    echo "[*] Installing BitNet service unit..."
    cat > "$SYSTEMD_DIR/bitnet.service" <<EOF
[Unit]
Description=BitNet Inference Server (OpenAI-compatible API)
After=network.target
StartLimitIntervalSec=60
StartLimitBurst=3

[Service]
Type=simple
WorkingDirectory=${BITNET_WORKDIR}
ExecStart=${BITNET_BIN} -m ${BITNET_MODEL_REL} --host 127.0.0.1 --port 8000
Restart=on-failure
RestartSec=10
TimeoutStartSec=120
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
EOF
fi

# Chain BitNet to start with speechd-ng if it is the selected backend
DROPIN_DIR="$SYSTEMD_DIR/speechd-ng.service.d"
DROPIN_FILE="$DROPIN_DIR/bitnet-chain.conf"
if [[ "${AI_BACKEND:-}" == "bitnet" || "${AI_BACKEND:-}" == "auto" ]]; then
    echo "[*] Chaining BitNet to speechd-ng startup..."
    mkdir -p "$DROPIN_DIR"
    cat > "$DROPIN_FILE" <<'DROPIN'
[Unit]
Wants=bitnet.service
After=bitnet.service
DROPIN
    systemctl --user enable bitnet 2>/dev/null || true
else
    if [ -f "$DROPIN_FILE" ]; then
        rm -f "$DROPIN_FILE"
        rmdir --ignore-fail-on-non-empty "$DROPIN_DIR"
    fi
fi

echo "[*] Enabling services..."
systemctl --user daemon-reload
systemctl --user enable --now speechd-ng

if [ "${VISION_INSTALLED:-false}" = true ]; then
    systemctl --user enable speechd-vision
    echo "    Vision service enabled (start with: systemctl --user start speechd-vision)"
fi

# ============================================================================
# Smoke Tests
# ============================================================================
echo ""
echo "--- Verifying Installation ---"
TESTS_PASSED=0
TESTS_FAILED=0

smoke_pass() { echo "  PASS: $1"; ((TESTS_PASSED++)); }
smoke_fail() { echo "  FAIL: $1"; ((TESTS_FAILED++)); }

# Test 1: speechd-ng binary exists
if [ -x "$BIN_DIR/speechd-ng" ]; then
    smoke_pass "speechd-ng binary installed"
else
    smoke_fail "speechd-ng binary missing at $BIN_DIR/speechd-ng"
fi

# Test 2: speechd-ng service is active
sleep 1
if systemctl --user is-active --quiet speechd-ng; then
    smoke_pass "speechd-ng.service is active"
else
    smoke_fail "speechd-ng.service is not active"
    systemctl --user status speechd-ng --no-pager -n 5 2>/dev/null || true
fi

# Test 3: D-Bus name registered
if dbus-send --session --print-reply --dest=org.freedesktop.DBus \
    /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null \
    | grep -q "org.speech.Service"; then
    smoke_pass "org.speech.Service registered on D-Bus"
else
    smoke_fail "org.speech.Service not found on D-Bus"
fi

# Test 4: TTS smoke test
if "$BIN_DIR/speechd-control" speak "Installation complete" 2>/dev/null; then
    smoke_pass "TTS spoke successfully"
else
    smoke_fail "TTS failed (check piper-tts is installed)"
fi

# Test 5: llama-server (only if BitNet is selected and we installed it)
if [[ "${AI_BACKEND:-}" == "bitnet" || "${AI_BACKEND:-}" == "auto" ]]; then
    LLAMA_BIN="${BITNET_BIN:-$BIN_DIR/llama-server}"
    if [ -x "$LLAMA_BIN" ]; then
        MISSING=$(LD_LIBRARY_PATH="${HOME}/.local/lib" ldd "$LLAMA_BIN" 2>/dev/null | grep "not found" || true)
        if [ -n "$MISSING" ]; then
            smoke_fail "llama-server has missing libraries: $(echo "$MISSING" | tr '\n' ' ')"
        else
            smoke_pass "llama-server shared libraries OK"
        fi
    else
        smoke_fail "llama-server binary not found at $LLAMA_BIN"
    fi

    # Test 6: bitnet.service starts and stays up
    systemctl --user restart bitnet 2>/dev/null || true
    sleep 5
    if systemctl --user is-active --quiet bitnet; then
        smoke_pass "bitnet.service is active"
    else
        STATUS=$(systemctl --user status bitnet --no-pager -n 3 2>/dev/null | tail -3)
        smoke_fail "bitnet.service failed to stay active: $STATUS"
    fi
fi

echo ""
echo "========================================"
if [ "$TESTS_FAILED" -eq 0 ]; then
    echo "   Installation Complete! ($TESTS_PASSED/$TESTS_PASSED tests passed)"
else
    echo "   Installation done with warnings ($TESTS_PASSED passed, $TESTS_FAILED failed)"
fi
echo "========================================"
echo ""
echo "Installed:"
echo "  - speechd-ng (core daemon) - RUNNING"
echo "  - speechd-control (CLI)"
if [ "${VISION_INSTALLED:-false}" = true ]; then
echo "  - speechd-vision (The Eye) - ENABLED"
fi
echo ""
echo "Commands:"
echo "  speechd-control speak 'Hello world'"
echo "  speechd-control listen"
echo "  speechd-control think 'What is the meaning of life?'"
if [ "${VISION_INSTALLED:-false}" = true ]; then
echo "  speechd-control describe 'What do you see?'"
fi
echo ""
echo "Services:"
echo "  systemctl --user status speechd-ng"
echo "  systemctl --user restart speechd-ng"
if [ "${VISION_INSTALLED:-false}" = true ]; then
echo "  systemctl --user start speechd-vision"
fi
echo ""
