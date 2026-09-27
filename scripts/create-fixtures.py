"""Generate original, low-level test sounds; never scan the user's collection."""
from pathlib import Path
import math
import random
import shutil
import struct
import wave

root = Path(__file__).resolve().parent.parent / 'test-fixtures' / 'sonidos'
root.mkdir(parents=True, exist_ok=True)
random.seed(42)

def write(name, duration, kind, channels=1):
    path = root / name
    if path.exists():
        return
    rate = 44100
    with wave.open(str(path), 'wb') as output:
        output.setparams((channels, 2, rate, 0, 'NONE', 'not compressed'))
        for start in range(0, int(duration * rate), 4096):
            chunk = bytearray()
            for i in range(start, min(start + 4096, int(duration * rate))):
                t = i / rate
                fade = min(1, t / .02, (duration - t) / .05)
                if kind == 'tone':
                    value = .09 * math.sin(2 * math.pi * 220 * t)
                elif kind == 'pulse':
                    value = .12 * math.sin(2 * math.pi * 65 * t) * math.exp(-18 * (t % .5))
                elif kind == 'noise':
                    value = .025 * random.uniform(-1, 1)
                else:
                    value = 0
                sample = struct.pack('<h', int(value * max(0, fade) * 32767))
                chunk.extend(sample * channels)
            output.writeframesraw(chunk)

write('01 · Pulso suave.wav', 8, 'pulse')
write('02 · Nota de referencia A3.wav', 12, 'tone', 2)
write('03 · Textura de ruido.wav', 5, 'noise', 2)
write('04 · Silencio.wav', 3, 'silence')
write('05 · Grabación larga.wav', 180, 'tone')
copy = root / '06 · Copia exacta del pulso.wav'
if not copy.exists():
    shutil.copyfile(root / '01 · Pulso suave.wav', copy)
corrupt = root / '07 · Archivo corrupto.wav'
if not corrupt.exists():
    corrupt.write_bytes(b'This is intentionally not an audio file.\n')
print(root)
