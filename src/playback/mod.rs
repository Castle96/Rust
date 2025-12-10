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

    // Optional: fetch artist general info (name, genre, url, etc.). Default: not supported.
    async fn artist_info(&mut self, artist_id: &str) -> Result<String> {
        Ok("artist info not supported by this adapter".into())
    }

    // Optional: fetch artist discography (albums). Default: not supported.
    async fn artist_discography(&mut self, artist_id: &str) -> Result<String> {
        Ok("artist discography not supported by this adapter".into())
    }
}

#[cfg(target_os = "macos")]
mod macos;

mod applemusic;
#[cfg(unix)]
mod mpv;
mod noop;
mod system;
mod applemusic_oauth;

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
