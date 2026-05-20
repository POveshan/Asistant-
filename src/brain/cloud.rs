use anyhow::{anyhow, Result};
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
        self.ask_with_history(prompt, &[]).await
    }

    pub async fn ask_with_history(
        &self,
        prompt: &str,
        history: &[(String, String)],
    ) -> Result<String> {
        if self.api_key.is_empty() {
            return Err(anyhow!("GROQ_API_KEY не установлен!"));
        }
        info!("🧠 Groq LLM: llama-3.3-70b-versatile (с историей)");

        // Собираем messages: system + история + текущий prompt
        let mut messages = vec![
            json!({"role": "system", "content": "Ты голосовой ассистент Шилов. Отвечай кратко по-русски. Помни контекст разговора."}),
        ];

        // Добавляем историю (последние 5 пар)
        for (user_msg, assistant_msg) in history.iter().rev().take(5).rev() {
            messages.push(json!({"role": "user", "content": user_msg}));
            messages.push(json!({"role": "assistant", "content": assistant_msg}));
        }

        // Текущий вопрос
        messages.push(json!({"role": "user", "content": prompt}));

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "llama-3.3-70b-versatile",
                "messages": messages,
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
            .as_str()
            .unwrap_or("...")
            .trim()
            .to_string();
        info!("🤖 Ответ: {}", text);
        Ok(text)
    }
}
