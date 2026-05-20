use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use tracing::warn;

use crate::system::KdeIntegration;

pub struct CommandRouter {
    kde: KdeIntegration,
    patterns: HashMap<String, Regex>,
}

impl CommandRouter {
    pub fn new(kde: KdeIntegration) -> Self {
        let mut patterns = HashMap::new();

        // Паттерны команд (точные фразы)
        patterns.insert(
            "terminal".to_string(),
            Regex::new(r"(терминал|консоль|konsole|terminal)").unwrap(),
        );
        patterns.insert(
            "browser".to_string(),
            Regex::new(r"(браузер|firefox|chrome|веб)").unwrap(),
        );
        patterns.insert(
            "open_terminal".to_string(),
            Regex::new(r"открой\s+(терминал|консоль|konsole)").unwrap(),
        );
        patterns.insert(
            "open_browser".to_string(),
            Regex::new(r"открой\s+(браузер|firefox|chrome|веб)").unwrap(),
        );
        patterns.insert(
            "volume".to_string(),
            Regex::new(r"(громкость|volume|звук)\s*(\d+)").unwrap(),
        );
        patterns.insert(
            "brightness".to_string(),
            Regex::new(r"(яркость|brightness)\s*(\d+)").unwrap(),
        );
        patterns.insert(
            "screenshot".to_string(),
            Regex::new(r"(скриншот|screenshot|снимок)").unwrap(),
        );
        patterns.insert(
            "lock".to_string(),
            Regex::new(r"(заблокируй|lock|блокировка)").unwrap(),
        );
        patterns.insert(
            "shutdown".to_string(),
            Regex::new(r"(выключи|shutdown|выключение)").unwrap(),
        );
        patterns.insert(
            "restart".to_string(),
            Regex::new(r"(перезагрузи|restart|ребут)").unwrap(),
        );
        patterns.insert(
            "time".to_string(),
            Regex::new(r"(время|time|который час)").unwrap(),
        );
        patterns.insert(
            "weather".to_string(),
            Regex::new(r"(погода|weather|прогноз)").unwrap(),
        );

        Self { kde, patterns }
    }

    pub async fn execute(&self, text: &str) -> Result<String> {
        let text_lower = text.to_lowercase();

        // Сначала проверяем точные фразы "открой ..."
        if self.patterns["open_terminal"].is_match(&text_lower) {
            self.kde.launch_app("konsole").await?;
            return Ok("Терминал открыт".to_string());
        }

        if self.patterns["open_browser"].is_match(&text_lower) {
            let browser = std::env::var("BROWSER").unwrap_or_else(|_| "firefox".to_string());
            self.kde.launch_app(&browser).await?;
            return Ok(format!("Браузер {} открыт", browser));
        }

        // Потом общие паттерны
        if self.patterns["terminal"].is_match(&text_lower) {
            self.kde.launch_app("konsole").await?;
            return Ok("Терминал открыт".to_string());
        }

        if self.patterns["browser"].is_match(&text_lower) {
            let browser = std::env::var("BROWSER").unwrap_or_else(|_| "firefox".to_string());
            self.kde.launch_app(&browser).await?;
            return Ok(format!("Браузер {} открыт", browser));
        }

        if let Some(caps) = self.patterns["volume"].captures(&text_lower) {
            let percent: u8 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(50);
            self.kde.set_volume(percent).await?;
            return Ok(format!("Громкость установлена на {}%", percent));
        }

        if let Some(caps) = self.patterns["brightness"].captures(&text_lower) {
            let percent: u8 = caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(50);
            self.kde.set_brightness(percent).await?;
            return Ok(format!("Яркость установлена на {}%", percent));
        }

        if self.patterns["screenshot"].is_match(&text_lower) {
            self.kde.screenshot().await?;
            return Ok("Скриншот сохранён".to_string());
        }

        if self.patterns["lock"].is_match(&text_lower) {
            self.kde.lock_screen().await?;
            return Ok("Экран заблокирован".to_string());
        }

        if self.patterns["shutdown"].is_match(&text_lower) {
            warn!("Запрошено выключение");
            self.kde.shutdown().await?;
            return Ok("Система выключается".to_string());
        }

        if self.patterns["restart"].is_match(&text_lower) {
            warn!("Запрошена перезагрузка");
            self.kde.restart().await?;
            return Ok("Система перезагружается".to_string());
        }

        if self.patterns["time"].is_match(&text_lower) {
            let now = chrono::Local::now();
            return Ok(format!("Сейчас {}", now.format("%H:%M")));
        }

        if self.patterns["weather"].is_match(&text_lower) {
            return Err(anyhow!(
                "Погода пока не реализована — отправлю в Groq позже"
            ));
        }

        Err(anyhow!("Команда не распознана: '{}'", text))
    }
}
