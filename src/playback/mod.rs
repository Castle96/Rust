use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait PlaybackAdapter {
    async fn search(&mut self, query: &str) -> Result<String>;
    async fn play(&mut self, track_id: Option<&str>) -> Result<()>;
    async fn pause(&mut self) -> Result<()>;
    async fn next(&mut self) -> Result<()>;
    async fn prev(&mut self) -> Result<()>;
    async fn status(&mut self) -> Result<String>;

    // Volume control (0-100). Default: not supported.
    async fn volume_up(&mut self) -> Result<()> {
        Err(anyhow::anyhow!(
            "volume control not supported by this adapter"
        ))
    }

    async fn volume_down(&mut self) -> Result<()> {
        Err(anyhow::anyhow!(
            "volume control not supported by this adapter"
        ))
    }

    async fn set_volume(&mut self, _volume: u8) -> Result<()> {
        Err(anyhow::anyhow!(
            "volume control not supported by this adapter"
        ))
    }

    async fn get_volume(&mut self) -> Result<u8> {
        Err(anyhow::anyhow!(
            "volume control not supported by this adapter"
        ))
    }

    async fn mute(&mut self) -> Result<()> {
        Err(anyhow::anyhow!("mute not supported by this adapter"))
    }

    async fn unmute(&mut self) -> Result<()> {
        Err(anyhow::anyhow!("unmute not supported by this adapter"))
    }

    // Seek control (seconds). Default: not supported.
    async fn seek_forward(&mut self, _seconds: u64) -> Result<()> {
        Err(anyhow::anyhow!("seek not supported by this adapter"))
    }

    async fn seek_backward(&mut self, _seconds: u64) -> Result<()> {
        Err(anyhow::anyhow!("seek not supported by this adapter"))
    }

    async fn seek_to(&mut self, _seconds: u64) -> Result<()> {
        Err(anyhow::anyhow!("seek not supported by this adapter"))
    }

    async fn get_position(&mut self) -> Result<u64> {
        Err(anyhow::anyhow!("position not supported by this adapter"))
    }

    async fn get_duration(&mut self) -> Result<u64> {
        Err(anyhow::anyhow!("duration not supported by this adapter"))
    }

    // Optional: fetch artist general info (name, genre, url, etc.). Default: not supported.
    async fn artist_info(&mut self, _artist_id: &str) -> Result<String> {
        Ok("artist info not supported by this adapter".into())
    }

    // Optional: fetch artist discography (albums). Default: not supported.
    async fn artist_discography(&mut self, _artist_id: &str) -> Result<String> {
        Ok("artist discography not supported by this adapter".into())
    }
}

#[cfg(target_os = "macos")]
mod macos;

mod applemusic;
mod applemusic_oauth;
#[cfg(unix)]
mod mpv;
mod noop;
mod system;

#[cfg(target_os = "macos")]
pub use macos::MacOsAdapter;

pub use applemusic::AppleMusicAdapter;
#[cfg(unix)]
pub use mpv::MpvAdapter;
pub use noop::NoopAdapter;
pub use system::SystemAdapter;

pub async fn get_adapter() -> Result<Box<dyn PlaybackAdapter + Send>> {
    // Allow selecting AppleMusic stub via env var APPLE_ADAPTER=applemusic
    if std::env::var("APPLE_ADAPTER")
        .map(|v| v == "applemusic")
        .unwrap_or(false)
    {
        return Ok(Box::new(AppleMusicAdapter::new()));
    }

    // Prefer mpv on unix
    #[cfg(unix)]
    {
        if let Ok(adapter) = MpvAdapter::try_new().await {
            return Ok(Box::new(adapter));
        }
    }

    if let Ok(adapter) = SystemAdapter::try_new() {
        return Ok(Box::new(adapter));
    }

    #[cfg(target_os = "macos")]
    {
        return Ok(Box::new(MacOsAdapter::new()));
    }

    Ok(Box::new(NoopAdapter::new()))
}
