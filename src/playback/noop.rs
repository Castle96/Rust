use crate::playback::PlaybackAdapter;
use anyhow::Result;

pub struct NoopAdapter {}

impl NoopAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for NoopAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PlaybackAdapter for NoopAdapter {
    async fn search(&mut self, query: &str) -> Result<String> {
        Ok(format!(
            "noop: search '{}': no results (not on macOS)",
            query
        ))
    }

    async fn play(&mut self, track_id: Option<&str>) -> Result<()> {
        println!("noop: play {:?}", track_id);
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        println!("noop: pause");
        Ok(())
    }

    async fn next(&mut self) -> Result<()> {
        println!("noop: next");
        Ok(())
    }

    async fn prev(&mut self) -> Result<()> {
        println!("noop: prev");
        Ok(())
    }

    async fn status(&mut self) -> Result<String> {
        Ok("noop: no status".to_string())
    }
}
