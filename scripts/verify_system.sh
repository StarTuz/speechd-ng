#!/bin/bash
set -e

echo "==========================================="
echo "   SpeechD-NG End-to-End Verification"
echo "==========================================="
echo ""

# 1. Check Core Daemon
echo "[*] Checking Core Daemon (speechd-ng)..."
if systemctl --user is-active --quiet speechd-ng; then
    echo "    [PASS] Systemd service is active."
else
    echo "    [FAIL] Systemd service is NOT active!"
    systemctl --user status speechd-ng --no-pager
    exit 1
fi

# 2. Check D-Bus Connectivity
echo "[*] Pinging via D-Bus..."
PING_RESPONSE=$(busctl --user call org.speech.Service /org/speech/Service org.speech.Service Ping)
if [[ "$PING_RESPONSE" == *"pong"* ]]; then
    echo "    [PASS] Core D-Bus API is responsive."
else
    echo "    [FAIL] D-Bus Ping failed: $PING_RESPONSE"
    exit 1
fi

# 3. Check Privacy (Microphone Gate)
echo "[*] Verifying Privacy Setting..."
# We check the config file directly for the definitive ground truth
MIC_SETTING=$(grep "enable_microphone" $HOME/.config/speechd-ng/Speech.toml || echo "enable_microphone = true")
if [[ "$MIC_SETTING" == *"false"* ]]; then
     echo "    [PASS] Microphone is HARD-DISABLED in config."
else
     echo "    [WARN] Microphone is ENABLED in config."
fi

# 4. Check Vision (The Eye)
echo "[*] Checking Vision Service..."
VISION_CONFIG=$(grep "enable_vision" $HOME/.config/speechd-ng/Speech.toml || echo "enable_vision = false")

if [[ "$VISION_CONFIG" == *"true"* ]]; then
    echo "    [INFO] Vision is ENABLED in config."
    if systemctl --user is-active --quiet speechd-vision; then
        echo "    [PASS] Vision service is running."
        echo "    [TEST] Sending 'PreloadModel' request..."
        # This tests the D-Bus link to vision without capturing a screenshot
        VISION_PING=$(busctl --user call org.speech.Vision /org/speech/Vision org.speech.Vision PreloadModel)
        echo "    [PASS] Vision D-Bus responded: $VISION_PING"
    else
        echo "    [FAIL] Vision is enabled but service is NOT running."
         exit 1
    fi
else
    echo "    [PASS] Vision is DISABLED (Modular-by-default verified)."
    if systemctl --user is-active --quiet speechd-vision; then
        echo "    [WARN] Vision service is running but disabled in config (Waste of resources)."
    fi
fi

echo ""
echo "==========================================="
echo "   [SUCCESS] E2E Verification Passed"
echo "==========================================="
