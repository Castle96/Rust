use crate::playback::PlaybackAdapter;
use anyhow::Result;
use std::collections::VecDeque;

pub struct Player {
    queue: VecDeque<String>,
    adapter: Box<dyn PlaybackAdapter + Send>,
}

impl Player {
    pub fn new(adapter: Box<dyn PlaybackAdapter + Send>) -> Self {
        Self {
            queue: VecDeque::new(),
            adapter,
        }
    }

    pub fn enqueue(&mut self, item: String) {
        self.queue.push_back(item);
    }

    pub fn list(&self) -> Vec<String> {
        self.queue.iter().cloned().collect()
    }

    pub fn next_item(&mut self) -> Option<String> {
        self.queue.pop_front()
    }

    pub async fn play_item(&mut self, item: &str) -> Result<()> {
        self.adapter.play(Some(item)).await?;
        Ok(())
    }

    pub fn adapter_mut(&mut self) -> &mut (dyn PlaybackAdapter + Send) {
        &mut *self.adapter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAdapter;

    #[async_trait::async_trait]
    impl PlaybackAdapter for MockAdapter {
        async fn search(&mut self, _query: &str) -> anyhow::Result<String> {
            Ok("".into())
        }
        async fn play(&mut self, _track_id: Option<&str>) -> anyhow::Result<()> {
            Ok(())
        }
        async fn pause(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        async fn next(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        async fn prev(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        async fn status(&mut self) -> anyhow::Result<String> {
            Ok("".into())
        }
    }

    #[tokio::test]
    async fn queue_basic() {
        let mock = Box::new(MockAdapter);
        let mut player = Player::new(mock);
        player.enqueue("one".into());
        player.enqueue("two".into());
        assert_eq!(player.list(), vec!["one".to_string(), "two".to_string()]);
        assert_eq!(player.next_item(), Some("one".to_string()));
    }
}
