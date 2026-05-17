use anyhow::{anyhow, Result};
use std::process::Command;
use tracing::{info, warn};

pub struct TtsEngine {
    venv_python: String,
}

impl TtsEngine {
    pub fn new() -> Self {
        let venv_python = format!(
            "{}/.local/share/kuro-tts/bin/python3",
            std::env::var("HOME").unwrap_or_default()
        );

        info!("🔊 TTS: Silero (через Python venv)");
        Self { venv_python }
    }

    pub async fn speak(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        info!("🔊 Говорю: {}", text);

        let safe_text = text.replace("'", "\\'").replace("\"", "\\\"");

        let script = format!(
            r#"
import torch
import soundfile as sf
import subprocess

device = torch.device('cpu')
model, _ = torch.hub.load('snakers4/silero-models', 'silero_tts', language='ru', speaker='v5_ru', trust_repo=True)
audio = model.apply_tts(text='{}', speaker='kseniya', sample_rate=48000)
sf.write('/tmp/kuro_tts_raw.wav', audio.numpy(), 48000)

result = subprocess.run(['sox', '/tmp/kuro_tts_raw.wav', '/tmp/kuro_tts_output.wav', 'pitch', '300'], capture_output=True)
if result.returncode != 0:
    import shutil
    shutil.copy('/tmp/kuro_tts_raw.wav', '/tmp/kuro_tts_output.wav')
"#,
            safe_text
        );

        let output = Command::new(&self.venv_python)
            .arg("-c")
            .arg(&script)
            .output()
            .map_err(|e| anyhow!("Python TTS не запустился: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            warn!("TTS ошибка: {}", err);
            let _ = Command::new("ffplay")
                .args(&["-nodisp", "-autoexit", "/tmp/kuro_tts_raw.wav"])
                .status();
            return Ok(());
        }

        let play_status = Command::new("ffplay")
            .args(&[
                "-nodisp",
                "-autoexit",
                "-volume",
                "100",
                "/tmp/kuro_tts_output.wav",
            ])
            .status();

        if play_status.is_err() || !play_status.unwrap().success() {
            let _ = Command::new("aplay")
                .arg("/tmp/kuro_tts_output.wav")
                .status();
        }

        Ok(())
    }
}
