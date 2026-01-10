# SpeechD-NG: The Future of Desktop Multimodal Awareness

## 🎙️ What is SpeechD-NG?

SpeechD-NG is a **Privacy-First, Multimodal Speech Assistant Daemon** for the Linux Desktop. It is a unified service that bridges the gap between raw AI models and your everyday workflow. It provides:

- **Hearing**: High-performance local Speech-to-Text (STT) [Opt-in].
- **Thinking**: Context-aware local LLM reasoning (via Ollama) [Opt-in].
- **Seeing**: Local Computer Vision for screen analysis (The Eye) [Optional Module].
- **Speaking**: High-quality Neural Text-to-Speech (TTS).

## 🚀 Why install and make use of it?

1. **Unparalleled Privacy**: 100% local inference. Microphones and cameras are **HARD-DISABLED** by default.
2. **Situational Intelligence**: Can "see" your screen, but only when you explicitly ask.
3. **Passive Learning**: Adapts to your voice over time (if enabled).
4. **Systems Grade Reliability**: Written in pure Rust with strict E2E verification (`verify_system.sh`).

## ⚔️ Is there anything like it on Linux?

**Strictly speaking: No.**

While individual tools exist for parts of the stack, SpeechD-NG is the only "Multimodal VUI Daemon":

- **Vs. Speech-Dispatcher**: The classic dispatcher is 20 years old and only handles TTS routing. It has no STT, no LLM context, and no vision.
- **Vs. Mycroft/Neon**: These are full environment replacements or heavy application suites. SpeechD-NG is a **lightweight daemon** that integrates into *any* existing Desktop Environment (GNOME, KDE, Sway).
- **Vs. Cloud Assistants (Alexa/Siri)**: SpeechD-NG provides the same capabilities but with zero latency, zero subscription fees, and total physical data control.

---
*Created by The Council of Experts.*
