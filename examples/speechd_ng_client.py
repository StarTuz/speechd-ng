#!/usr/bin/env python3
"""
SpeechD-NG Python Client Library

A simple wrapper for the SpeechD-NG D-Bus API.

Usage:
    from speechd_ng_client import SpeechClient
    
    client = SpeechClient()
    client.speak("Hello world")
    client.add_correction("mozurt", "mozart")
    
Requirements:
    pip install dbus-python
"""

import dbus
from typing import List, Tuple, Optional


class SpeechClient:
    """
    Client for interacting with the SpeechD-NG daemon via D-Bus.
    """
    
    SERVICE = 'org.speech.Service'
    PATH = '/org/speech/Service'
    INTERFACE = 'org.speech.Service'
    
    def __init__(self):
        """Initialize connection to SpeechD-NG."""
        self.bus = dbus.SessionBus()
        self._service = self.bus.get_object(self.SERVICE, self.PATH)
        self._iface = dbus.Interface(self._service, self.INTERFACE)
    
    # ========== TTS Methods ==========
    
    def speak(self, text: str, voice: Optional[str] = None):
        """
        Speak text using TTS.
        
        Args:
            text: Text to speak
            voice: Optional voice ID (e.g., "en_US-lessac-medium")
        """
        if voice:
            self._iface.SpeakVoice(text, voice)
        else:
            self._iface.Speak(text)
    
    def list_voices(self) -> List[Tuple[str, str]]:
        """
        List all installed voices.
        
        Returns:
            List of (voice_id, display_name) tuples
        """
        return [(str(vid), str(name)) for vid, name in self._iface.ListVoices()]
    
    def list_downloadable_voices(self) -> List[Tuple[str, str]]:
        """
        List voices available for download.
        
        Returns:
            List of (voice_id, description) tuples
        """
        return [(str(vid), str(desc)) for vid, desc in self._iface.ListDownloadableVoices()]
    
    def download_voice(self, voice_id: str) -> str:
        """
        Download a Piper neural voice.
        
        Args:
            voice_id: Voice ID (e.g., "piper:en_US-amy-low")
            
        Returns:
            "Success" or error message
        """
        return str(self._iface.DownloadVoice(voice_id))
    
    # ========== AI Methods ==========
    
    def think(self, query: str) -> str:
        """
        Ask the AI about recent speech context.
        
        Args:
            query: Question to ask
            
        Returns:
            AI-generated response
        """
        return str(self._iface.Think(query))
    
    def listen(self) -> str:
        """
        Record audio and transcribe.
        
        Returns:
            Transcribed text
        """
        return str(self._iface.Listen())
    
    # ========== Training Methods (Phase 9) ==========
    
    def add_correction(self, heard: str, meant: str) -> bool:
        """
        Add a manual voice correction pattern.
        
        Args:
            heard: What ASR incorrectly hears
            meant: What the user actually said
            
        Returns:
            True if pattern was added
        """
        return bool(self._iface.AddCorrection(heard, meant))
    
    def train_word(self, expected: str, duration_secs: int = 3) -> Tuple[str, bool]:
        """
        Record audio and learn what ASR hears for a word.
        
        Args:
            expected: What the user intends to say
            duration_secs: Recording duration
            
        Returns:
            Tuple of (what_asr_heard, success)
        """
        heard, success = self._iface.TrainWord(expected, duration_secs)
        return (str(heard), bool(success))
    
    def list_patterns(self) -> List[Tuple[str, str, str]]:
        """
        List all learned voice patterns.
        
        Returns:
            List of (heard, meant, confidence_info) tuples
        """
        return [(str(h), str(m), str(c)) for h, m, c in self._iface.ListPatterns()]
    
    def get_fingerprint_stats(self) -> dict:
        """
        Get fingerprint statistics.
        
        Returns:
            Dict with 'manual', 'passive', and 'commands' counts
        """
        manual, passive, commands = self._iface.GetFingerprintStats()
        return {
            'manual': int(manual),
            'passive': int(passive),
            'commands': int(commands)
        }
    
    # ========== Import/Export Methods (Phase 10) ==========
    
    def export_fingerprint(self, path: str) -> bool:
        """
        Export learned patterns to a file.
        
        Args:
            path: Absolute path (must be in writable location)
            
        Returns:
            True if successful
        """
        return bool(self._iface.ExportFingerprint(path))
    
    def import_fingerprint(self, path: str, merge: bool = True) -> int:
        """
        Import patterns from a file.
        
        Args:
            path: Absolute path to import file
            merge: If True, adds without overwriting existing
            
        Returns:
            Total pattern count after import
        """
        return int(self._iface.ImportFingerprint(path, merge))
    
    def get_fingerprint_path(self) -> str:
        """
        Get the path to the fingerprint data file.
        
        Returns:
            Path string
        """
        return str(self._iface.GetFingerprintPath())
    
    # ========== Ignored Commands Methods (Phase 11) ==========
    
    def get_ignored_commands(self) -> List[Tuple[str, str, str]]:
        """
        Get all unrecognized/failed ASR attempts.
        
        Returns:
            List of (heard, timestamp, context) tuples
        """
        return [(str(h), str(t), str(c)) for h, t, c in self._iface.GetIgnoredCommands()]
    
    def clear_ignored_commands(self) -> int:
        """
        Clear all ignored commands.
        
        Returns:
            Count of commands cleared
        """
        return int(self._iface.ClearIgnoredCommands())
    
    def correct_ignored_command(self, heard: str, meant: str) -> bool:
        """
        Correct an ignored command and add as pattern.
        
        Args:
            heard: The ignored ASR transcription
            meant: What the user actually intended
            
        Returns:
            True if command was found and corrected
        """
        return bool(self._iface.CorrectIgnoredCommand(heard, meant))
    
    def add_ignored_command(self, heard: str, context: str = ""):
        """
        Manually add a command to the ignored list.
        
        Args:
            heard: The unrecognized text
            context: Optional context about why it was ignored
        """
        self._iface.AddIgnoredCommand(heard, context)


# ========== Example Usage ==========

if __name__ == "__main__":
    print("SpeechD-NG Python Client Demo")
    print("=" * 40)
    
    try:
        client = SpeechClient()
    except dbus.exceptions.DBusException as e:
        print(f"Error: Could not connect to SpeechD-NG: {e}")
        print("Is the service running? Try: systemctl --user status speechd-ng")
        exit(1)
    
    # Show stats
    stats = client.get_fingerprint_stats()
    print(f"\nLearning Stats:")
    print(f"  Manual patterns: {stats['manual']}")
    print(f"  Passive patterns: {stats['passive']}")
    print(f"  Command history: {stats['commands']}")
    
    # Show patterns
    patterns = client.list_patterns()
    if patterns:
        print(f"\nLearned Patterns ({len(patterns)}):")
        for heard, meant, conf in patterns[:5]:  # Show first 5
            print(f"  '{heard}' → '{meant}' ({conf})")
        if len(patterns) > 5:
            print(f"  ... and {len(patterns) - 5} more")
    else:
        print("\nNo patterns learned yet.")
    
    # Show ignored commands
    ignored = client.get_ignored_commands()
    if ignored:
        print(f"\nIgnored Commands ({len(ignored)}):")
        for heard, timestamp, context in ignored[:3]:  # Show first 3
            print(f"  '{heard}' at {timestamp} ({context})")
    else:
        print("\nNo ignored commands.")
    
    # Speak
    print("\nSpeaking greeting...")
    client.speak("Hello! SpeechD-NG is working correctly.")
    
    print("\nDemo complete!")
