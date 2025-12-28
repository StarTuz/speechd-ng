#!/usr/bin/env python3
import sys
import json
import os
from vosk import Model, KaldiRecognizer

def main():
    if len(sys.argv) < 3:
        print("Usage: wakeword.py <model_path> <keyword>")
        sys.exit(1)

    model_path = sys.argv[1]
    keyword = sys.argv[2].lower()

    if not os.path.exists(model_path):
        print(f"Error: Model not found at {model_path}", file=sys.stderr)
        sys.exit(1)

    model = Model(model_path)
    # 16000Hz is standard for small vosk models
    rec = KaldiRecognizer(model, 16000)

    # Read raw PCM from stdin
    while True:
        data = sys.stdin.buffer.read(4000)
        if len(data) == 0:
            break
        if rec.AcceptWaveform(data):
            res = json.loads(rec.Result())
            text = res.get("text", "").lower()
            if keyword in text:
                print("DETECTED")
                sys.stdout.flush()

if __name__ == "__main__":
    main()
