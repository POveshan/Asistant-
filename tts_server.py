#!/usr/bin/env python3
import sys
import torch
import soundfile as sf
import tempfile
import os

# Загружаем модель ОДИН РАЗ при старте
print("Загрузка Silero TTS v5...", file=sys.stderr)
device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
model, _ = torch.hub.load('snakers4/silero-models',
                          'silero_tts',
                          language='ru',
                          speaker='v5_ru',
                          trust_repo=True)
model.to(device)
print(f"Silero v5 готов! Устройство: {device}", file=sys.stderr)

def speak(text):
    audio = model.apply_tts(text=text,
                            speaker='kseniya',
                            sample_rate=48000,
                            put_accent=True,
                            put_yo=True)
    
    with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as f:
        sf.write(f.name, audio.numpy(), 48000)
        return f.name

if __name__ == '__main__':
    if len(sys.argv) > 1:
        # Режим одноразового запуска: tts_server.py "текст"
        text = sys.argv[1]
        try:
            wav_path = speak(text)
            print(wav_path)
        except Exception as e:
            print(f"ERROR: {e}", file=sys.stderr)
            sys.exit(1)
    else:
        # Режим сервера: читаем stdin
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            if line == "__EXIT__":
                break
            try:
                wav_path = speak(line)
                print(wav_path, flush=True)
            except Exception as e:
                print(f"ERROR: {e}", file=sys.stderr, flush=True)
                print("", flush=True)
