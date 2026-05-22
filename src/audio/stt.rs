use anyhow::{anyhow, Result};
use std::process::Command;
use tracing::{error, info};

pub struct SttEngine {
    api_key: String,
    proxy: String,
}

impl SttEngine {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("GROQ_API_KEY")
            .unwrap_or_else(|_| {
                info!("⚠️ GROQ_API_KEY не найден, STT работать не будет");
                String::new()
            });
        
        let proxy = "socks5h://127.0.0.1:2080".to_string();
        
        Ok(Self { api_key, proxy })
    }
    
    /// Основной метод распознавания (для совместимости с capture.rs)
    pub async fn recognize_file(&self, wav_path: &str) -> Result<String> {
        self.transcribe(wav_path).await
    }
    
    /// Распознавание речи через Groq Whisper API
    pub async fn transcribe(&self, wav_path: &str) -> Result<String> {
        if self.api_key.is_empty() {
            return Err(anyhow!("GROQ_API_KEY не установлен"));
        }
        
        info!("🔍 Отправляю аудио в Groq Whisper через curl...");
        
        let output = Command::new("curl")
            .args(&[
                "--proxy", &self.proxy,
                "--connect-timeout", "30",
                "--max-time", "60",
                "-s", "-X", "POST",
                "https://api.groq.com/openai/v1/audio/transcriptions",
                "-H", &format!("Authorization: Bearer {}", self.api_key),
                "-H", "Content-Type: multipart/form-data",
                "-F", "model=whisper-large-v3",
                "-F", &format!("file=@{}", wav_path),
            ])
            .output()?;
        
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            error!("❌ Curl ошибка: {}", err);
            return Err(anyhow!("Curl failed: {}", err));
        }
        
        let response = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&response)?;
        
        let text = json["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();
        
        if text.is_empty() {
            return Err(anyhow!("Пустой ответ от API"));
        }
        
        info!("📝 Распознано: '{}'", text);
        Ok(text)
    }
}
