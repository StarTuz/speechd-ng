#!/usr/bin/env python3
"""
SpeechD-NG Python Client Example
--------------------------------
Demonstrates how to interact with the SpeechD-NG D-Bus service.
"""
import dbus
import sys
import os

class SpeechClient:
    def __init__(self):
        try:
            self.bus = dbus.SessionBus()
            self.service = self.bus.get_object('org.speech.Service', '/org/speech/Service')
            self.iface = dbus.Interface(self.service, 'org.speech.Service')
            print("Connected to SpeechD-NG service.")
        except dbus.exceptions.DBusException as e:
            print(f"Error connecting to service: {e}")
            sys.exit(1)
    
    # --- TTS ---
    def speak(self, text, voice=None):
        if voice:
            print(f"Speaking with voice '{voice}': {text}")
            self.iface.SpeakVoice(text, voice)
        else:
            print(f"Speaking: {text}")
            self.iface.Speak(text)

    def list_voices(self):
        return self.iface.ListVoices()

    # --- AI ---
    def think(self, query):
        print(f"Thinking on: {query}")
        response = self.iface.Think(query)
        print(f"AI Response: {response}")
        return response

    def listen(self):
        print("Listening (Manual)...")
        response = self.iface.Listen()
        print(f"Heard: {response}")
        return response
        
    def listen_vad(self):
        print("Listening (VAD)...")
        response = self.iface.ListenVad()
        print(f"Heard: {response}")
        return response

    # --- Training ---
    def train_word(self, word, duration=3):
        print(f"Training word '{word}' for {duration}s...")
        heard, success = self.iface.TrainWord(word, duration)
        print(f"Result: heard='{heard}', success={success}")
        return success

    def add_correction(self, heard, meant):
        print(f"Adding correction: '{heard}' -> '{meant}'")
        if self.iface.AddCorrection(heard, meant):
            print("Success.")
        else:
            print("Failed.")

    def list_patterns(self):
        patterns = self.iface.ListPatterns()
        print(f"Found {len(patterns)} patterns:")
        for heard, meant, conf in patterns:
            print(f"  - '{heard}' -> '{meant}' [{conf}]")

    def get_stats(self):
        m, p, c = self.iface.GetFingerprintStats()
        print(f"Stats: Manual={m}, Passive={p}, Commands={c}")

    # --- Ignored Commands ---
    def get_ignored(self):
        ignored = self.iface.GetIgnoredCommands()
        print(f"Ignored Commands ({len(ignored)}):")
        for h, t, ctx in ignored:
            print(f"  - '{h}' (at {t}) [Context: {ctx}]")

    # --- Configuration Phase 13 ---
    def get_config(self):
        backend = self.iface.GetSttBackend()
        host, port, model, auto = self.iface.GetWyomingInfo()
        print("Configuration:")
        print(f"  STT Backend: {backend}")
        print(f"  Wyoming: {host}:{port} (Model: {model})")


def print_menu():
    print("\n--- SpeechD-NG Client ---")
    print("1. Speak")
    print("2. Listen (Fixed 5s)")
    print("3. Listen (VAD)")
    print("4. Think (ask AI)")
    print("5. List Voices")
    print("6. Show Learning Stats")
    print("7. List Patterns")
    print("8. Show Ignored Commands")
    print("9. Show Configuration")
    print("0. Exit")

if __name__ == "__main__":
    client = SpeechClient()
    
    if len(sys.argv) > 1:
        # Quick CLI mode
        cmd = sys.argv[1]
        if cmd == "speak":
            client.speak(" ".join(sys.argv[2:]))
        elif cmd == "listen":
            client.listen_vad()
        elif cmd == "config":
            client.get_config()
        elif cmd == "stats":
            client.get_stats()
        elif cmd == "patterns":
            client.list_patterns()
        elif cmd == "ignored":
            client.get_ignored()
        else:
            print(f"Unknown command: {cmd}")
        sys.exit(0)

    while True:
        print_menu()
        choice = input("Select: ")
        
        if choice == "0": break
        elif choice == "1":
            text = input("Text to speak: ")
            client.speak(text)
        elif choice == "2":
            client.listen()
        elif choice == "3":
            client.listen_vad()
        elif choice == "4":
            q = input("Query: ")
            client.think(q)
        elif choice == "5":
            voices = client.list_voices()
            for vid, name in voices:
                print(f" - {name} ({vid})")
        elif choice == "6":
            client.get_stats()
        elif choice == "7":
            client.list_patterns()
        elif choice == "8":
            client.get_ignored()
        elif choice == "9":
            client.get_config()
        else:
            print("Invalid option")
