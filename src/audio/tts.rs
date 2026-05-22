use anyhow::Result;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TtsEngine {
    inner: Arc<TtsEngineInner>,
}

struct TtsEngineInner {
    process: Mutex<std::process::Child>,
    stdin: Mutex<std::process::ChildStdin>,
    stdout: Mutex<BufReader<std::process::ChildStdout>>,
}

const PID_FILE: &str = "/tmp/kuro_tts.pid";

impl TtsEngine {
    pub fn new() -> Result<Self> {
        if Path::new(PID_FILE).exists() {
            println!("Подключаемся к существующему TTS серверу...");
        }

        let mut process = Command::new("/home/kde_user/step_counter/.venv_tts/bin/python")
            .arg("/home/kde_user/step_counter/tts_server.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let pid = process.id();
        std::fs::write(PID_FILE, pid.to_string())?;

        let stdin = process.stdin.take().unwrap();
        let stdout = BufReader::new(process.stdout.take().unwrap());

        Ok(Self {
            inner: Arc::new(TtsEngineInner {
                process: Mutex::new(process),
                stdin: Mutex::new(stdin),
                stdout: Mutex::new(stdout),
            }),
        })
    }

    pub fn speak(&self, text: &str) -> Result<String> {
        let mut stdin = self.inner.stdin.lock().unwrap();
        writeln!(stdin, "{}", text)?;
        stdin.flush()?;

        let mut stdout = self.inner.stdout.lock().unwrap();
        let mut response = String::new();
        stdout.read_line(&mut response)?;
        let wav_path = response.trim().to_string();

        if wav_path.is_empty() {
            anyhow::bail!("TTS ошибка генерации");
        }

        Ok(wav_path)
    }

    pub fn stop_speaking(&self) {
        let _ = Command::new("pkill").args(&["-9", "ffplay"]).status();
        let _ = Command::new("pkill").args(&["-9", "aplay"]).status();
    }

    pub fn play_audio(&self, wav_path: &str) -> Result<()> {
        let status = Command::new("ffplay")
            .args(&["-nodisp", "-autoexit", wav_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            Command::new("aplay")
                .arg(wav_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
        }

        Ok(())
    }
}

impl Drop for TtsEngineInner {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin.get_mut().unwrap(), "__EXIT__");
        let _ = self.stdin.get_mut().unwrap().flush();
        let _ = self.process.get_mut().unwrap().wait();
        let _ = std::fs::remove_file(PID_FILE);
    }
}
