#!/usr/bin/env python3
"""
Wyoming ASR Bridge for SpeechD-NG

Bridges audio from Rust to Wyoming protocol servers (wyoming-faster-whisper).
Reads raw 16-bit PCM audio from stdin, sends to Wyoming server, outputs transcript to stdout.

Usage:
    echo "audio_data" | python wyoming_bridge.py --host 127.0.0.1 --port 10301

Protocol:
    - Input: Raw 16-bit PCM audio at 16000Hz mono via stdin
    - Output: "TRANSCRIPT: <text>" on stdout when transcription is ready
    - Signals: "READY" when connected, "ERROR: <msg>" on failure
"""

import sys
import argparse
import asyncio
import struct
print("DEBUG: Bridge starting...", file=sys.stderr)

# Silence Wyoming library logging
import logging
for logger_name in ["wyoming", "wyoming.event", "wyoming.client", "wyoming.server", "wyoming.audio"]:
    logging.getLogger(logger_name).setLevel(logging.CRITICAL)
    logging.getLogger(logger_name).disabled = True

try:
    from wyoming.client import AsyncTcpClient
    from wyoming.info import Describe
    from wyoming.audio import AudioStart, AudioChunk, AudioStop
    from wyoming.asr import Transcript
except ImportError:
    print("ERROR: Wyoming not installed. Run: pip install wyoming", file=sys.stderr)
    sys.exit(1)


class WyomingBridge:
    def __init__(self, host: str, port: int, sample_rate: int = 16000):
        self.host = host
        self.port = port
        self.sample_rate = sample_rate
        self.chunk_size = 1024  # ~64ms at 16kHz
        self.client = None
        
    async def connect(self) -> bool:
        """Connect to Wyoming server."""
        try:
            self.client = AsyncTcpClient(self.host, self.port)
            await self.client.connect()
            
            # Handshake
            await self.client.write_event(Describe().event())
            event = await self.client.read_event()
            
            if event:
                print(f"CONNECTED: {event.type}", flush=True)
                return True
            return False
            
        except Exception as e:
            print(f"ERROR: Connection failed - {e}", file=sys.stderr)
            return False
    
    async def transcribe_from_stdin(self) -> str:
        """Read audio from stdin and transcribe via Wyoming."""
        if not self.client:
            return ""
        
        try:
            print("DEBUG: Sending AudioStart", file=sys.stderr)
            # Send AudioStart
            audio_start = AudioStart(
                rate=self.sample_rate,
                width=2,  # 16-bit
                channels=1
            )
            await self.client.write_event(audio_start.event())
            print("DEBUG: AudioStart sent", file=sys.stderr)
            
            # Read audio from stdin and send chunks
            chunks_sent = 0
            chunk_duration_ms = (self.chunk_size / self.sample_rate) * 1000.0
            
            while True:
                # Read raw PCM data
                data = sys.stdin.buffer.read(self.chunk_size * 2)  # 2 bytes per sample
                
                if not data:
                    break
                
                # Send chunk
                timestamp = int(chunks_sent * chunk_duration_ms)
                chunk = AudioChunk(
                    rate=self.sample_rate,
                    width=2,
                    channels=1,
                    audio=data,
                    timestamp=timestamp
                )
                await self.client.write_event(chunk.event())
                chunks_sent += 1
            
            # Send AudioStop
            await self.client.write_event(AudioStop().event())
            
            # Wait for transcript
            while True:
                event = await asyncio.wait_for(
                    self.client.read_event(),
                    timeout=30.0
                )
                
                if not event:
                    break
                    
                print(f"DEBUG RX: {event.type}", file=sys.stderr)
                
                if Transcript.is_type(event.type):
                    transcript = Transcript.from_event(event)
                    return transcript.text.strip()
                    
        except asyncio.TimeoutError:
            print("ERROR: Transcription timeout", file=sys.stderr)
        except Exception as e:
            print(f"ERROR: Transcription failed - {e}", file=sys.stderr)
        
        return ""
    
    async def disconnect(self):
        """Close connection."""
        if self.client:
            try:
                await self.client.disconnect()
            except:
                pass
            self.client = None


async def main():
    parser = argparse.ArgumentParser(description="Wyoming ASR Bridge")
    parser.add_argument("--host", default="127.0.0.1", help="Wyoming server host")
    parser.add_argument("--port", type=int, default=10301, help="Wyoming server port")
    parser.add_argument("--rate", type=int, default=16000, help="Audio sample rate")
    args = parser.parse_args()
    
    bridge = WyomingBridge(args.host, args.port, args.rate)
    
    if not await bridge.connect():
        print("ERROR: Failed to connect to Wyoming server")
        sys.exit(1)
    
    print("READY", flush=True)
    
    transcript = await bridge.transcribe_from_stdin()
    
    if transcript:
        print(f"TRANSCRIPT: {transcript}", flush=True)
    else:
        print("TRANSCRIPT:", flush=True)
    
    await bridge.disconnect()


if __name__ == "__main__":
    asyncio.run(main())
