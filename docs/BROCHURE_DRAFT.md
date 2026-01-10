# SpeechD-NG: The Future of Desktop Multimodal Awareness

## 🎙️ What is SpeechD-NG?

SpeechD-NG is a **Privacy-First, Multimodal Speech Assistant Daemon** for the Linux Desktop. It is a unified service that bridges the gap between raw AI models and your everyday workflow. It provides:

- **Hearing**: High-performance local Speech-to-Text (STT).
- **Thinking**: Context-aware local LLM reasoning (via Ollama).
- **Seeing**: Local Computer Vision for screen analysis (The Eye).
- **Speaking**: High-quality Neural Text-to-Speech (TTS).

## 🚀 Why install and make use of it?

1. **Unparalleled Privacy**: 100% local inference. Your screen, your voice, and your data never leave your machine.
2. **Situational Intelligence**: Unlike generic assistants, it can "see" what you are doing. Ask: *"What's the error in this terminal?"* or *"Explain this graph"*.
3. **Self-Improving Phonetics**: Features **Passive Learning**—it remembers your unique accent and corrections, improving accuracy the more you use it.
4. **Systems Grade Reliability**: Written in pure Rust with strict Systemd sandboxing, zero Python dependencies, and sub-millisecond D-Bus response times.

## ⚔️ Is there anything like it on Linux?

**Strictly speaking: No.**

While individual tools exist for parts of the stack, SpeechD-NG is the only "Multimodal VUI Daemon":

- **Vs. Speech-Dispatcher**: The classic dispatcher is 20 years old and only handles TTS routing. It has no STT, no LLM context, and no vision.
- **Vs. Mycroft/Neon**: These are full environment replacements or heavy application suites. SpeechD-NG is a **lightweight daemon** that integrates into *any* existing Desktop Environment (GNOME, KDE, Sway).
- **Vs. Cloud Assistants (Alexa/Siri)**: SpeechD-NG provides the same capabilities but with zero latency, zero subscription fees, and total physical data control.

---
*Created by The Council of Experts.*
