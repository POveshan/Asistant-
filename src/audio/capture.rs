use anyhow::Result;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
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
        info!("▶️ Аудиозахват: VAD + фильтр шума");

        loop {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let raw_path = format!("/tmp/kuro_raw_{}.wav", timestamp);

            // parec -> sox: highpass/lowpass режут шум + silence VAD
            let record = Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "parec --rate=48000 --channels=1 --format=s16le 2>/dev/null | \
                     sox -t raw -r 48000 -c 1 -b 16 -e signed - {} \
                     highpass 200 lowpass 4000 \
                     silence 1 0.1 1% 1 0.5 1%",
                    raw_path
                ))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            let success = record.map(|s| s.success()).unwrap_or(false);

            // Fallback: arecord
            let success = if !success {
                Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "arecord -D plughw:0,0 -r 48000 -c 1 -f S16_LE -t raw 2>/dev/null | \
                         sox -t raw -r 48000 -c 1 -b 16 -e signed - {} \
                         highpass 200 lowpass 4000 \
                         silence 1 0.1 1% 1 0.5 1%",
                        raw_path
                    ))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else {
                true
            };

            if !success {
                sleep(Duration::from_millis(300)).await;
                continue;
            }

            let metadata = tokio::fs::metadata(&raw_path).await;
            if metadata.is_err() || metadata.unwrap().len() < 2000 {
                let _ = tokio::fs::remove_file(&raw_path).await;
                sleep(Duration::from_millis(200)).await;
                continue;
            }

            // Проверяем: шум или нормальная речь?
            let stat_output = Command::new("sox")
                .args([&raw_path, "-n", "stat"])
                .stderr(Stdio::piped())
                .output();

            let mut max_amp = 0.0f32;
            let mut length_sec = 0.0f32;
            if let Ok(output) = stat_output {
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    if line.contains("Maximum amplitude") {
                        if let Some(val) = line.split_whitespace().last() {
                            if let Ok(v) = val.parse::<f32>() {
                                max_amp = v;
                            }
                        }
                    }
                    if line.contains("Length (seconds)") {
                        if let Some(val) = line.split_whitespace().last() {
                            if let Ok(v) = val.parse::<f32>() {
                                length_sec = v;
                            }
                        }
                    }
                }
            }

            info!(
                "📊 Амплитуда: {:.3}, Длительность: {:.2} сек",
                max_amp, length_sec
            );

            if max_amp < 0.10 {
                info!("🔇 Слишком тихо (шум?), игнорирую");
                let _ = tokio::fs::remove_file(&raw_path).await;
                sleep(Duration::from_millis(200)).await;
                continue;
            }

            if length_sec < 0.4 {
                info!("🔇 Слишком коротко (щелчок?), игнорирую");
                let _ = tokio::fs::remove_file(&raw_path).await;
                sleep(Duration::from_millis(200)).await;
                continue;
            }

            match self.stt.recognize_file(&raw_path).await {
                Ok(text) => {
                    if !text.is_empty() {
                        let _ = tx.send(text).await;
                    }
                }
                Err(e) => {
                    warn!("❌ Ошибка распознавания: {}", e);
                }
            }

            let _ = tokio::fs::remove_file(&raw_path).await;
            sleep(Duration::from_millis(200)).await;
        }
    }
}
