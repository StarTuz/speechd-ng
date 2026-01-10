#!/usr/bin/env python3
"""
SpeechD-NG Spatial Audio Demo
Showcases multi-channel audio capabilities (Phantom Center, Rear Surround, LFE).
"""

import dbus
import time
import sys

def main():
    try:
        bus = dbus.SessionBus()
        service = bus.get_object('org.speech.Service', '/org/speech/Service')
        iface = dbus.Interface(service, 'org.speech.Service')
    except Exception as e:
        print(f"Error: Could not connect to SpeechD-NG: {e}")
        sys.exit(1)

    print("--- SpeechD-NG Spatial Audio Demo ---")
    
    # 1. List available channels
    try:
        channels = iface.ListChannels()
        print("\nAvailable Channels:")
        for name, desc in channels:
            print(f" - {name:15}: {desc}")
    except:
        print("\nNote: ListChannels() not yet implemented or error.")

    print("\n--- Testing Stereo Panning ---")
    
    print("Panning Left...")
    iface.SpeakChannel("This is coming from your left speaker.", "", "left")
    time.sleep(2)
    
    print("Panning Right...")
    iface.SpeakChannel("And this is coming from your right speaker.", "", "right")
    time.sleep(2)
    
    print("Phantom Center...")
    iface.SpeakChannel("This is the phantom center, mixed seventy percent into both channels.", "", "center")
    time.sleep(3)

    print("\n--- Testing 5.1 Surround (If supported by hardware) ---")
    
    print("Rear Left...")
    iface.SpeakChannel("Rear left. Traffic behind you to the left.", "", "rear-left")
    time.sleep(3)
    
    print("Rear Right...")
    iface.SpeakChannel("Rear right. Traffic behind you to the right.", "", "rear-right")
    time.sleep(3)
    
    print("Subwoofer (LFE)...")
    iface.SpeakChannel("LFE. Testing the low frequency effects channel.", "", "lfe")
    time.sleep(3)

    print("\n--- Testing Concurrent Playback ---")
    # Using PlayAudio for a URL to show it doesn't block
    url = "https://www.soundjay.com/buttons/beep-01a.wav"
    print(f"Playing background effect to center: {url}")
    iface.PlayAudioChannel(url, "center")
    
    print("Speaking while audio plays...")
    iface.Speak("I can speak while the background effect is playing because the refactored engine is non-blocking.")
    
    print("\nDemo Complete.")

if __name__ == "__main__":
    main()
