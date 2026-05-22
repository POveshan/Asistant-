#!/usr/bin/env python3
import sys
import tempfile

import soundfile as sf
import torch

print("Загрузка Silero TTS v4...", file=sys.stderr)
device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

# v4_ru — мультиспикерная модель, API отличается от v5!
# Возвращает кортеж: (model, symbols, sample_rate, example_text, apply_tts)
model, symbols, sample_rate, example_text, apply_tts = torch.hub.load(
    "snakers4/silero-models",
    "silero_tts",
    language="ru",
    speaker="v4_ru",  # <-- Загружаем мультиспикерную модель
    trust_repo=True,
)

model.to(device)
print(f"Silero v4 готов! Устройство: {device}", file=sys.stderr)

# Меняй здесь: 'kseniya', 'baya', 'irina', 'natasha', 'tatyana'
SPEAKER = "xenia"


def speak(text):
    # В v4 используем apply_tts, а не model.apply_tts
    # И передаем model и sample_rate явно
    audio = apply_tts(
        text=text,
        model=model,
        sample_rate=sample_rate,
        speaker=SPEAKER,  # <-- Здесь голос реально меняется!
        put_accent=True,
        put_yo=True,
    )

    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        sf.write(f.name, audio, sample_rate)
        return f.name


if __name__ == "__main__":
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
