use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::info;

pub struct CloudBrain {
    client: Client,
    api_key: String,
}

impl CloudBrain {
    pub fn new() -> Self {
        let api_key = std::env::var("GROQ_API_KEY").unwrap_or_default();
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        Self { client, api_key }
    }

    pub async fn ask(&self, prompt: &str) -> Result<String> {
        if self.api_key.is_empty() {
            return Err(anyhow!("GROQ_API_KEY не установлен!"));
        }
        info!("🧠 Groq LLM: llama-3.3-70b-versatile");
        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "llama-3.3-70b-versatile",
                "messages": [
                    {"role": "system", "content": "Ты голосовой ассистент Куро. Отвечай кратко по-русски."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.7,
                "max_tokens": 150
            }))
            .send()
            .await
            .map_err(|e| anyhow!("Groq недоступен: {}", e))?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(anyhow!("Groq ошибка {}: {}", status, text));
        }
        let json: serde_json::Value = response.json().await?;
        let text = json["choices"][0]["message"]["content"]
            .as_str().unwrap_or("...").trim().to_string();
        info!("🤖 Ответ: {}", text);
        Ok(text)
    }
}
