use anyhow::{anyhow, Result};
use reqwest::Client;

use std::time::Duration;
use tracing::{info, warn};

pub struct SttEngine {
    client: Client,
    api_key: String,
}

impl Clone for SttEngine {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
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

        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;

        info!("🧠 STT: Groq Whisper API");
        Ok(Self { client, api_key })
    }

    pub async fn process_chunk(&self, _chunk: &[f32]) -> Result<Option<String>> {
        Ok(None)
    }

    pub async fn recognize_file(&self, wav_path: &str) -> Result<String> {
        info!("🔍 Отправляю аудио в Groq Whisper...");

        let file_bytes = tokio::fs::read(wav_path).await?;

        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(file_bytes)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")?,
            )
            .text("model", "whisper-large-v3")
            .text("language", "ru")
            .text("response_format", "text");

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| anyhow!("Groq Whisper недоступен: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let err = response.text().await?;
            return Err(anyhow!("Groq Whisper ошибка {}: {}", status, err));
        }

        let text = response.text().await?.trim().to_string();
        info!("📝 Распознано: '{}'", text);
        Ok(text)
    }
}
