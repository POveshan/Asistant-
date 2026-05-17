use anyhow::Result;

/// Piper TTS (заглушка)
pub struct PiperTts;

impl PiperTts {
    pub fn new() -> Self {
        Self
    }

    pub async fn speak(&self, _text: &str) -> Result<()> {
        Ok(())
    }
}
