use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};
use std::time::Duration;
use tracing::{info, warn};

pub struct SttEngine {
    api_key: String,
}

impl Clone for SttEngine {
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
        }
    }
}

impl SttEngine {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("GROQ_API_KEY").unwrap_or_default();

        if api_key.is_empty() {
            warn!("GROQ_API_KEY не установлен!");
        }

        info!("🧠 STT: Groq Whisper API (через curl)");
        Ok(Self { api_key })
    }

    pub async fn process_chunk(&self, _chunk: &[f32]) -> Result<Option<String>> {
        Ok(None)
    }

    pub async fn recognize_file(&self, wav_path: &str) -> Result<String> {
        info!("🔍 Отправляю аудио в Groq Whisper через curl...");

        // Используем curl с socks5 прокси NekoBox
        let output = Command::new("curl")
            .args(&[
                "-s",
                "-x",
                "socks5h://127.0.0.1:2080",
                "--connect-timeout",
                "30",
                "--max-time",
                "60",
                "-X",
                "POST",
                "https://api.groq.com/openai/v1/audio/transcriptions",
                "-H",
                &format!("Authorization: Bearer {}", self.api_key),
                "-F",
                "model=whisper-large-v3",
                "-F",
                "language=ru",
                "-F",
                "response_format=text",
                "-F",
                &format!("file=@{}", wav_path),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow!("curl не запустился: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(anyhow!("Groq ошибка: {} | stderr: {}", stdout, stderr));
        }

        // Проверяем, что ответ не JSON с ошибкой
        if stdout.trim().starts_with('{') {
            return Err(anyhow!("Groq API ошибка: {}", stdout));
        }

        let text = stdout.trim().to_string();
        info!("📝 Распознано: '{}'", text);
        Ok(text)
    }
}
