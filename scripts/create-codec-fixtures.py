"""Optional developer-only codec corpus. FFmpeg is NOT a runtime dependency."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
source = root / 'test-fixtures/sonidos/02 · Nota de referencia A3.wav'
if not source.exists():
    raise SystemExit('Run npm run fixtures first')
destination = root / 'test-fixtures/codecs'
destination.mkdir(parents=True, exist_ok=True)
for extension, codec in [('wav','pcm_s16le'), ('aiff','pcm_s16be'), ('flac','flac'), ('mp3','libmp3lame'), ('m4a','aac'), ('aac','aac'), ('ogg','vorbis')]:
    output = destination / ('reference.' + extension)
    if output.exists():
        continue
    subprocess.run(['ffmpeg', '-v', 'error', '-n', '-i', str(source), '-c:a', codec, '-strict', '-2', str(output)], check=True)
print(destination)
