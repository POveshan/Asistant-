use anyhow::{Result, anyhow};
use std::process::Command;
use tracing::{info, warn};
use zbus::{Connection, proxy};

/// D-Bus прокси для KDE Plasma
#[proxy(
    interface = "org.freedesktop.ScreenSaver",
    default_service = "org.freedesktop.ScreenSaver",
    default_path = "/ScreenSaver"
)]
trait ScreenSaver {
    fn lock(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.kde.KWin",
    default_service = "org.kde.KWin",
    default_path = "/KWin"
)]
trait KWin {
    fn active_window(&self) -> zbus::Result<String>;
}

pub struct KdeIntegration {
    dbus: Connection,
}

impl KdeIntegration {
    pub async fn new() -> Result<Self> {
        let dbus = Connection::session().await?;
        info!("✅ D-Bus подключен (KDE Plasma)");
        Ok(Self { dbus })
    }

    /// Запуск приложения
    pub async fn launch_app(&self, app: &str) -> Result<()> {
        info!("🚀 Запуск: {}", app);

        // Пробуем через kstart5 (KDE way)
        let status = Command::new("kstart5")
            .arg(app)
            .spawn()
            .map(|mut child| child.wait())
            .map_err(|e| anyhow!("Не удалось запустить {}: {}", app, e))?;

        // Fallback на прямой запуск
        if status.is_err() || !status.unwrap().success() {
            Command::new(app)
                .spawn()
                .map_err(|e| anyhow!("Fallback запуск {} провален: {}", app, e))?;
        }

        Ok(())
    }

    /// Установка громкости через amixer/pactl
    pub async fn set_volume(&self, percent: u8) -> Result<()> {
        let percent = percent.clamp(0, 100);
        info!("🔊 Громкость: {}%", percent);

        // Пробуем pactl (PipeWire/PulseAudio)
        let output = Command::new("pactl")
            .args(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", percent)])
            .output();

        if output.is_err() {
            // Fallback на amixer
            Command::new("amixer")
                .args(&["set", "Master", &format!("{}%", percent)])
                .output()
                .map_err(|e| anyhow!("Не удалось установить громкость: {}", e))?;
        }

        Ok(())
    }

    /// Установка яркости через brightnessctl или xbacklight
    pub async fn set_brightness(&self, percent: u8) -> Result<()> {
        let percent = percent.clamp(0, 100);
        info!("💡 Яркость: {}%", percent);

        let output = Command::new("brightnessctl")
            .args(&["set", &format!("{}%", percent)])
            .output();

        if output.is_err() {
            Command::new("xbacklight")
                .args(&["-set", &percent.to_string()])
                .output()
                .map_err(|e| anyhow!("Не удалось установить яркость: {}", e))?;
        }

        Ok(())
    }

    /// Скриншот через spectacle (KDE) или grim (Wayland)
    pub async fn screenshot(&self) -> Result<()> {
        info!("📸 Скриншот");

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("~/Pictures/screenshot_{}.png", timestamp);

        // KDE spectacle
        let result = Command::new("spectacle")
            .args(&["-b", "-o", &filename])
            .output();

        if result.is_err() {
            // Wayland grim
            Command::new("grim")
                .arg(&filename)
                .output()
                .map_err(|e| anyhow!("Не удалось сделать скриншот: {}", e))?;
        }

        Ok(())
    }

    /// Блокировка экрана через D-Bus
    pub async fn lock_screen(&self) -> Result<()> {
        info!("🔒 Блокировка экрана");

        let proxy = ScreenSaverProxy::new(&self.dbus).await?;
        proxy.lock().await?;

        Ok(())
    }

    /// Выключение системы
    pub async fn shutdown(&self) -> Result<()> {
        warn!("⚠️  Выключение системы!");

        Command::new("systemctl")
            .arg("poweroff")
            .spawn()
            .map_err(|e| anyhow!("Не удалось выключить: {}", e))?;

        Ok(())
    }

    /// Перезагрузка
    pub async fn restart(&self) -> Result<()> {
        warn!("⚠️  Перезагрузка системы!");

        Command::new("systemctl")
            .arg("reboot")
            .spawn()
            .map_err(|e| anyhow!("Не удалось перезагрузить: {}", e))?;

        Ok(())
    }

}
