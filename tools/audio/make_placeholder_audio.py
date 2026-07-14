"""Generate the template's placeholder sounds: assets/audio/*.wav.

Run from the repo root:

    python3 tools/audio/make_placeholder_audio.py

Pure-stdlib synthesis (16-bit mono 44.1 kHz), so the files carry no license
and regenerating them needs nothing but Python. Swap in real sounds by
replacing the files; the code only knows the paths.
"""

import math
import os
import struct
import wave

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "assets", "audio")
RATE = 44100


def write_wav(name, samples):
    path = os.path.join(OUT_DIR, name)
    with wave.open(path, "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(
            b"".join(struct.pack("<h", int(max(-1.0, min(1.0, s)) * 32767)) for s in samples)
        )
    print(f"wrote {path}")


def ui_click():
    """Short 880 Hz blip with a fast exponential decay."""
    length = int(RATE * 0.06)
    return [
        0.5 * math.sin(2 * math.pi * 880 * t / RATE) * math.exp(-t / (RATE * 0.012))
        for t in range(length)
    ]


def spatial_hum():
    """Loopable 110 Hz hum (+ a soft octave). Two seconds is an integer
    number of cycles for both partials, so the loop seam is silent."""
    length = int(RATE * 2.0)
    return [
        0.28 * math.sin(2 * math.pi * 110 * t / RATE)
        + 0.12 * math.sin(2 * math.pi * 220 * t / RATE)
        for t in range(length)
    ]


os.makedirs(OUT_DIR, exist_ok=True)
write_wav("ui_click.wav", ui_click())
write_wav("spatial_hum.wav", spatial_hum())
