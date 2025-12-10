use crate::playback::PlaybackAdapter;
use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

pub struct MacOsAdapter {}

impl MacOsAdapter {
    pub fn new() -> Self {
        Self {}
    }

    async fn run_applescript(script: &str) -> Result<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("failed to run osascript")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("osascript failed: {}", err);
        }

        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(out)
    }
}

#[async_trait::async_trait]
impl PlaybackAdapter for MacOsAdapter {
    async fn search(&mut self, query: &str) -> Result<String> {
        // Use Music app search via AppleScript: search library playlist for track
        let script = format!(
            r#"tell application \"Music\" to search (library playlist 1) for \"{}\""#,
            query
        );
        let res = Self::run_applescript(&script).await?;
        Ok(res)
    }

    async fn play(&mut self, track_id: Option<&str>) -> Result<()> {
        let script = if let Some(id) = track_id {
            // Play a track by persistent ID if provided
            format!(
                r#"tell application \"Music\" to play (every track of library playlist 1 whose persistent ID is \"{}\")"#,
                id
            )
        } else {
            "tell application \"Music\" to play".to_string()
        };
        Self::run_applescript(&script).await.map(|_| ())
    }

    async fn pause(&mut self) -> Result<()> {
        let script = "tell application \"Music\" to pause";
        Self::run_applescript(script).await.map(|_| ())
    }

    async fn next(&mut self) -> Result<()> {
        let script = "tell application \"Music\" to next track";
        Self::run_applescript(script).await.map(|_| ())
    }

    async fn prev(&mut self) -> Result<()> {
        let script = "tell application \"Music\" to previous track";
        Self::run_applescript(script).await.map(|_| ())
    }

    async fn status(&mut self) -> Result<String> {
        let script = r#"tell application \"Music\"
set t to current track
set name to name of t
set artist to artist of t
set playingState to player state
return name & " - " & artist & " (" & playingState & ")"
end tell"#;
        let res = Self::run_applescript(script).await?;
        Ok(res)
    }
}
