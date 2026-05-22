use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;
use tracing::{error, info};

pub struct CloudBrain {
    client: Client,
    api_key: String,
    model: String,
}

impl CloudBrain {
    pub fn new() -> Self {
        let api_key = std::env::var("GROQ_API_KEY").unwrap_or_else(|_| {
            info!("⚠️ GROQ_API_KEY не найден, ИИ работать не будет");
            String::new()
        });

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            api_key,
            model: "llama-3.3-70b-versatile".to_string(),
        }
    }

    pub async fn ask_with_history(
        &self,
        prompt: &str,
        history: &[(String, String)],
    ) -> Result<String> {
        if self.api_key.is_empty() {
            return Err(anyhow!("GROQ_API_KEY не установлен"));
        }

        info!("🧠 Groq LLM: {} (с историей)", self.model);

        let mut messages = vec![json!({
            "role": "system",
            "content": "Ты — голосовой ассистент Шилов. Отвечай кратко, по-русски, дружелюбно. Максимум 2-3 предложения."
        })];

        for (user_msg, assistant_msg) in history.iter().take(5) {
            messages.push(json!({"role": "user", "content": user_msg}));
            messages.push(json!({"role": "assistant", "content": assistant_msg}));
        }

        messages.push(json!({"role": "user", "content": prompt}));

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "max_tokens": 256,
                "temperature": 0.7
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("❌ Groq API ошибка {}: {}", status, text);
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let json: serde_json::Value = response.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Не понял вопрос")
            .trim()
            .to_string();

        info!("🤖 Ответ: {}", content);
        Ok(content)
    }
}
