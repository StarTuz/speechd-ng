#!/bin/bash
# Concurrency Collision Test Script
# Nikolai "Sprint" Volkov's Mandate

echo "🚀 Starting Concurrency Collision Test..."

# 1. Start a long TTS stream in the background
echo "📢 [1/3] Triggering long TTS stream..."
speechd-control speak "This is a long test sentence designed to keep the audio engine busy while we perform other heavy operations like screen capturing and model inference. We are monitoring for any audio stutter or jitter during this process. SpeechD-NG must handle multiple streams and multimodal queries without impacting the real-time playback quality." &
TTS_PID=$!

sleep 1

# 2. Trigger a screen description (Heavy Vision Load)
echo "👁️ [2/3] Triggering Screen Description (The Eye)..."
speechd-control describe "Explain the current desktop environment in detail" &
VISION_PID=$!

sleep 0.5

# 3. Simulate ASR activity
echo "👂 [3/3] Simulating ASR/Listen loop..."
speechd-control listen &
LISTEN_PID=$!

echo "⏳ Waiting for concurrent operations to complete..."
wait $VISION_PID
echo "✅ Vision inference complete."

wait $LISTEN_PID
echo "✅ Listen loop complete."

wait $TTS_PID
echo "✅ TTS stream complete."

echo "🏁 Concurrency Collision Test finished. Check logs for stutter reports."
