#!/usr/bin/env python3
"""
SpeechD-NG Latency Benchmark
----------------------------
Measures round-trip time for D-Bus methods.
"""
import dbus
import time
import sys
import statistics

class Benchmarker:
    def __init__(self):
        try:
            self.bus = dbus.SessionBus()
            self.service = self.bus.get_object('org.speech.Service', '/org/speech/Service')
            self.iface = dbus.Interface(self.service, 'org.speech.Service')
        except Exception as e:
            print(f"Failed to connect: {e}")
            sys.exit(1)

    def measure(self, name, func, iterations=5):
        times = []
        print(f"Benchmarking {name} ({iterations} iters)... ", end="", flush=True)
        for _ in range(iterations):
            start = time.perf_counter()
            func()
            end = time.perf_counter()
            times.append((end - start) * 1000.0) # ms
        
        avg = statistics.mean(times)
        p95 = statistics.quantiles(times, n=20)[18] if len(times) >= 20 else max(times)
        print(f"Avg: {avg:.2f}ms | Max: {max(times):.2f}ms")
        return avg

    def run(self):
        print("=== SpeechD-NG Latency Benchmark ===\n")

        # 0. Connection Check
        print("Pinging service... ", end="", flush=True)
        try:
            res = self.iface.Ping()
            print(f"[{res}]")
        except Exception as e:
            print(f"FAILED: {e}")
            sys.exit(1)

        # 1. Ping / Config (Baseline)
        self.measure("Ping (Internal)", lambda: self.iface.Ping(), iterations=50)
        self.measure("GetSttBackend (Config)", lambda: self.iface.GetSttBackend(), iterations=20)

        # 2. Speak (Fire and Forget)
        # This measures how fast the daemon creates the task, not audio duration.
        self.measure("Speak (IPC Overhead)", lambda: self.iface.Speak("Benchmark test"), iterations=10)

        # 3. Think (AI Round Trip)
        # This includes Ollama inference time if enabled.
        # Use a short prompt to minimize variance.
        try:
            self.measure("Think (Short Query)", lambda: self.iface.Think("Hi"), iterations=3)
        except Exception as e:
            print(f"Think failed (AI offline?): {e}")

        # 4. Pattern Stats (DB Access)
        self.measure("GetFingerprintStats (DB Read)", lambda: self.iface.GetFingerprintStats(), iterations=20)

        print("\n=== Done ===")

if __name__ == "__main__":
    Benchmarker().run()
