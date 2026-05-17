use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

mod audio;
mod brain;
mod commands;
mod system;

use audio::{tts::TtsEngine, AudioCapture, SttEngine};
use brain::cloud::CloudBrain;
use commands::CommandRouter;
use system::KdeIntegration;

#[derive(Debug, Clone, Default)]
pub struct KuroState {
    pub listening: bool,
    pub last_command: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,kuro_assistant=debug")
        .init();

    info!("🎌 Kuro Assistant запускается...");

    let state = Arc::new(RwLock::new(KuroState::default()));
    let kde = KdeIntegration::new().await?;
    let router = CommandRouter::new(kde);
    let stt = SttEngine::new()?;
    let capture = AudioCapture::new(stt)?;
    let tts = TtsEngine::new(); // ← ДОБАВИЛИ TTS

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

    let audio_task = async move {
        if let Err(e) = capture.run(tx).await {
            error!("Ошибка захвата аудио: {}", e);
        }
    };

    let router_handle = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            let text = text.to_lowercase().trim().to_string();
            info!("📝 Распознано: '{}'", text);

            {
                let mut state = state.write().await;
                state.last_command = Some(text.clone());
            }

            match router.execute(&text).await {
                Ok(result) => {
                    info!("✅ Выполнено: {}", result);
                    tts.speak(&result).await.ok(); // ← ГОВОРИМ результат команды
                }
                Err(_) => {
                    info!("🤖 Отправляю в Llama 3.3 70B: '{}'", text);
                    let brain = CloudBrain::new();
                    match brain.ask(&text).await {
                        Ok(response) => {
                            info!("🤖 Ответ: {}", response);
                            tts.speak(&response).await.ok(); // ← ГОВОРИМ ответ LLM
                        }
                        Err(e) => warn!("❌ Ошибка ИИ: {}", e),
                    }
                }
            }
        }
    });

    audio_task.await;
    router_handle.await?;

    Ok(())
}
