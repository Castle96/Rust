use crate::playback::PlaybackAdapter;
use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

pub struct SystemAdapter {
    player_cmd: Option<String>,
}

impl SystemAdapter {
    pub fn try_new() -> Result<Self> {
        // Check for mpv first; if not present, we'll fall back to system opener
        if which::which("mpv").is_ok() {
            Ok(Self {
                player_cmd: Some("mpv".to_string()),
            })
        } else {
            // fallback: no mpv found, we'll use system opener
            Ok(Self { player_cmd: None })
        }
    }

    async fn spawn_open(&self, target: &str) -> Result<()> {
        #[cfg(target_os = "linux")]
        let mut cmd = Command::new("xdg-open");
        #[cfg(target_os = "macos")]
        let mut cmd = Command::new("open");
        #[cfg(target_os = "windows")]
        let mut cmd = Command::new("cmd");

        #[cfg(target_os = "windows")]
        {
            cmd.arg("/C").arg("start").arg(target);
        }

        #[cfg(not(target_os = "windows"))]
        {
            cmd.arg(target);
        }

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        let _ = cmd.spawn().context("failed to spawn system opener")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PlaybackAdapter for SystemAdapter {
    async fn search(&mut self, query: &str) -> Result<String> {
        Ok(format!(
            "system: search '{}' (no remote search implemented)",
            query
        ))
    }

    async fn play(&mut self, track_id: Option<&str>) -> Result<()> {
        if let Some(id) = track_id {
            // If mpv present, play using mpv; otherwise open using system opener
            if let Some(cmd) = &self.player_cmd {
                let child = Command::new(cmd)
                    .arg("--no-video")
                    .arg(id)
                    .spawn()
                    .context("failed to spawn mpv")?;
                // don't await child; let it run
                let _ = child;
                Ok(())
            } else {
                self.spawn_open(id).await
            }
        } else {
            // no id: resume isn't supported by system opener; return Ok
            Ok(())
        }
    }

    async fn pause(&mut self) -> Result<()> {
        // We don't have a controller for system opener; if mpv is used we could implement IPC later
        Ok(())
    }

    async fn next(&mut self) -> Result<()> {
        Ok(())
    }

    async fn prev(&mut self) -> Result<()> {
        Ok(())
    }

    async fn status(&mut self) -> Result<String> {
        Ok("system: no status".to_string())
    }
}
