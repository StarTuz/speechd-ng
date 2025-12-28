#!/usr/bin/env python3
import sys
import json
import os
from vosk import Model, KaldiRecognizer

def main():
    if len(sys.argv) < 3:
        print("Usage: wakeword.py <model_path> <keyword> [sample_rate]")
        sys.exit(1)

    model_path = sys.argv[1]
    keyword = sys.argv[2].lower()
    sample_rate = int(sys.argv[3]) if len(sys.argv) > 3 else 16000

    if not os.path.exists(model_path):
        print(f"Error: Model not found at {model_path}", file=sys.stderr)
        sys.exit(1)

    model = Model(model_path)
    rec = KaldiRecognizer(model, sample_rate)

    # Read raw PCM from stdin
    print(f"Bridge: Started with keyword '{keyword}' at {sample_rate}Hz", file=sys.stderr)
    while True:
        data = sys.stdin.buffer.read(4000)
        if len(data) == 0:
            break
        
        if rec.AcceptWaveform(data):
            res = json.loads(rec.Result())
            text = res.get("text", "").lower()
            if text:
                print(f"Bridge: Recognized: '{text}'", file=sys.stderr)
            if keyword in text:
                print("DETECTED")
                sys.stdout.flush()
        else:
            partial = json.loads(rec.PartialResult())
            p_text = partial.get("partial", "").lower()
            if keyword in p_text:
                print("DETECTED")
                sys.stdout.flush()
                rec.Reset()

if __name__ == "__main__":
    main()
