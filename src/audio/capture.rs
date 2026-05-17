use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use ringbuf::{traits::*, HeapRb};
use tracing::{info, warn};

use super::stt::SttEngine;

pub struct AudioCapture {
    stt: SttEngine,
}

impl AudioCapture {
    pub fn new(stt: SttEngine) -> Result<Self> {
        info!("🎤 AudioCapture инициализирован");
        Ok(Self { stt })
    }

    pub async fn run(self, tx: tokio::sync::mpsc::Sender<String>) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .input_devices()?
            .find(|d| {
                d.name()
                    .map(|n| n.contains("USB") || n.contains("usb") || n.contains("Microphone"))
                    .unwrap_or(false)
            })
            .unwrap_or_else(|| host.default_input_device().unwrap());

        let device_name = device.name().unwrap_or_default();
        info!("🎤 Используется микрофон: {}", device_name);

        // Форсируем 48000 Hz — реальная частота USB микрофона
        let sample_rate = 48000u32;
        let sample_format = SampleFormat::F32;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        info!(
            "📊 Формат: {:?}, Sample rate: {} Hz (форсировано)",
            sample_format, sample_rate
        );

        let ring = HeapRb::<f32>::new(sample_rate as usize * 60);
        let (mut producer, mut consumer) = ring.split();

        let err_fn = |err| eprintln!("Ошибка аудиопотока: {}", err);

        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    for &sample in data {
                        if producer.try_push(sample).is_err() {
                            eprintln!("⚠️ Аудиобуфер переполнен!");
                        }
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    for &sample in data {
                        let sample = sample as f32 / i16::MAX as f32;
                        if producer.try_push(sample).is_err() {
                            eprintln!("⚠️ Аудиобуфер переполнен!");
                        }
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    for &sample in data {
                        let sample = (sample as f32 / u16::MAX as f32) * 2.0 - 1.0;
                        if producer.try_push(sample).is_err() {
                            eprintln!("⚠️ Аудиобуфер переполнен!");
                        }
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err(anyhow!("Формат {:?} не поддерживается", sample_format)),
        };

        stream.play()?;
        info!("▶️  Аудиопоток запущен. Говорите...");

        let mut audio_buffer: Vec<f32> = Vec::new();
        let mut silence_frames: u32 = 0;
        let silence_threshold = 0.05;
        let silence_frames_required = 15;
        let min_audio_duration = sample_rate as usize;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let mut chunk = Vec::new();
            while let Some(sample) = consumer.try_pop() {
                chunk.push(sample);
            }

            if chunk.is_empty() {
                continue;
            }

            let rms: f32 = chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32;
            let is_speech = rms > silence_threshold;

            if is_speech {
                audio_buffer.extend_from_slice(&chunk);
                silence_frames = 0;
            } else {
                if !audio_buffer.is_empty() {
                    audio_buffer.extend_from_slice(&chunk);
                    silence_frames += 1;

                    if silence_frames >= silence_frames_required
                        && audio_buffer.len() >= min_audio_duration
                    {
                        info!(
                            "🎤 Фраза завершена, буфер: {} сэмплов ({:.1} сек)",
                            audio_buffer.len(),
                            audio_buffer.len() as f32 / sample_rate as f32
                        );

                        let temp_path = format!(
                            "/tmp/kuro_audio_{}.wav",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis()
                        );

                        info!("💾 Сохраняю WAV (48000 Hz): {}", temp_path);
                        self.write_wav(&temp_path, &audio_buffer).await?;
                        info!("✅ WAV сохранён");

                        match self.stt.recognize_file(&temp_path).await {
                            Ok(text) => {
                                if !text.is_empty() {
                                    info!("📍 Распознано: '{}'", text);
                                    let _ = tx.send(text).await;
                                }
                            }
                            Err(e) => {
                                warn!("❌ Ошибка распознавания: {}", e);
                            }
                        }

                        audio_buffer.clear();
                        silence_frames = 0;
                    }
                }
            }

            if audio_buffer.len() > sample_rate as usize * 30 {
                warn!("⚠️ Буфер переполнен (30 сек), форсирую сохранение");

                let temp_path = format!(
                    "/tmp/kuro_audio_forced_{}.wav",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                );

                self.write_wav(&temp_path, &audio_buffer).await?;

                match self.stt.recognize_file(&temp_path).await {
                    Ok(text) if !text.is_empty() => {
                        let _ = tx.send(text).await;
                    }
                    _ => {}
                }

                audio_buffer.clear();
                silence_frames = 0;
                let _ = tokio::fs::remove_file(&temp_path).await;
            }
        }
    }

    async fn write_wav(&self, path: &str, samples: &[f32]) -> Result<()> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(int_sample)?;
        }

        writer.finalize()?;
        Ok(())
    }
}
