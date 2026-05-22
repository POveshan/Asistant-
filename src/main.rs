use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
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
    pub last_command: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,kuro_assistant=debug")
        .init();

    info!("🎌 Шилов Ассистент запускается...");
    info!("🎤 Скажи 'Шилов' чтобы разбудить");

    let state = Arc::new(Mutex::new(KuroState::default()));
    let kde = KdeIntegration::new().await?;
    let router = CommandRouter::new(kde);
    let stt = SttEngine::new()?;
    let capture = AudioCapture::new(stt)?;
    let tts = TtsEngine::new()?;

    let is_speaking = Arc::new(Mutex::new(false));
    let is_awake = Arc::new(Mutex::new(false));
    let dialog_history = Arc::new(Mutex::new(Vec::<(String, String)>::new()));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);

    let audio_handle = tokio::spawn(async move {
        if let Err(e) = capture.run(tx).await {
            error!("Ошибка захвата аудио: {}", e);
        }
    });

    let is_speaking_clone = Arc::clone(&is_speaking);
    let is_awake_clone = Arc::clone(&is_awake);
    let dialog_history_clone = Arc::clone(&dialog_history);
    let tts_clone_main = tts.clone();

    let router_handle = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            let lower = text.to_lowercase();

            // Если ассистент говорит — новый запрос прерывает речь
            if *is_speaking_clone.lock().await {
                let stop_words = ["хватит", "прекрати", "заткнись", "стоп", "тихо", "замолчи"];
                if stop_words.iter().any(|&w| lower.contains(w)) {
                    info!("🛑 Стоп-слово! Прерываю...");
                    tts_clone_main.stop_speaking();
                    *is_speaking_clone.lock().await = false;
                    continue;
                }

                info!("🛑 Новый запрос! Прерываю текущую речь...");
                tts_clone_main.stop_speaking();
                *is_speaking_clone.lock().await = false;
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }

            let text = lower.trim().to_string();
            info!("📝 Распознано: '{}'", text);

            let clean_text;
            let should_process;

            // Wake word логика
            {
                let mut awake = is_awake_clone.lock().await;
                if !*awake {
                    if text.contains("шилов") || text.contains("шил") {
                        *awake = true;

                        clean_text = text
                            .replace("шилов", "")
                            .replace("шил", "")
                            .trim()
                            .to_string();

                        if clean_text.is_empty() {
                            info!("🌅 Шилов проснулся!");
                            let tts_clone = tts.clone();
                            tokio::spawn(async move {
                                match tts_clone.speak("Да, я здесь") {
                                    Ok(wav_path) => {
                                        let _ = tts_clone.play_audio(&wav_path);
                                    }
                                    Err(e) => error!("TTS ошибка: {}", e),
                                }
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            });
                            continue;
                        }

                        info!("🌅 Шилов проснулся и сразу выполняет: '{}'", clean_text);
                        should_process = true;
                    } else {
                        info!("🤐 Шилов спит, скажи 'Шилов' чтобы разбудить");
                        continue;
                    }
                } else {
                    clean_text = text
                        .replace("шилов", "")
                        .replace("шил", "")
                        .trim()
                        .to_string();

                    if clean_text.is_empty() {
                        continue;
                    }
                    should_process = true;
                }
            }

            if !should_process || clean_text.is_empty() {
                continue;
            }

            info!("🎯 Обрабатываю: '{}'", clean_text);

            {
                let mut s = state.lock().await;
                s.last_command = Some(clean_text.clone());
            }

            *is_speaking_clone.lock().await = true;

            let history = {
                let h = dialog_history_clone.lock().await;
                h.clone()
            };

            match router.execute(&clean_text).await {
                Ok(cmd_result) => {
                    info!("✅ Выполнено: {}", cmd_result);

                    {
                        let mut h = dialog_history_clone.lock().await;
                        h.push((clean_text.clone(), cmd_result.clone()));
                        if h.len() > 10 {
                            h.remove(0);
                        }
                    }

                    let tts_clone = tts.clone();
                    let text_clone = cmd_result;
                    let speaking_flag = Arc::clone(&is_speaking_clone);
                    tokio::spawn(async move {
                        match tts_clone.speak(&text_clone) {
                            Ok(wav_path) => {
                                let _ = tts_clone.play_audio(&wav_path);
                            }
                            Err(e) => error!("TTS ошибка: {}", e),
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        *speaking_flag.lock().await = false;
                    });
                }
                Err(_) => {
                    info!("🤖 Отправляю в Llama 3.3 70B: '{}'", clean_text);
                    let brain = CloudBrain::new();

                    match brain.ask_with_history(&clean_text, &history).await {
                        Ok(response) => {
                            info!("🤖 Ответ: {}", response);

                            {
                                let mut h = dialog_history_clone.lock().await;
                                h.push((clean_text.clone(), response.clone()));
                                if h.len() > 10 {
                                    h.remove(0);
                                }
                            }

                            let tts_clone = tts.clone();
                            let text_clone = response;
                            let speaking_flag = Arc::clone(&is_speaking_clone);
                            tokio::spawn(async move {
                                match tts_clone.speak(&text_clone) {
                                    Ok(wav_path) => {
                                        let _ = tts_clone.play_audio(&wav_path);
                                    }
                                    Err(e) => error!("TTS ошибка: {}", e),
                                }
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                *speaking_flag.lock().await = false;
                            });
                        }
                        Err(e) => {
                            warn!("❌ Ошибка ИИ: {}", e);
                            *is_speaking_clone.lock().await = false;
                        }
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = audio_handle => {},
        _ = router_handle => {},
    }

    Ok(())
}
