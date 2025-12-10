use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::player::Player;

#[derive(Parser)]
#[command(name = "apple-music-cli")]
#[command(about = "Control Apple Music from the terminal", long_about = None)]
pub struct Cli {
    /// Run in daemon mode and accept commands over a local socket
    #[arg(long)]
    daemon: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Search {
        query: String,
    },
    Play {
        track_id: Option<String>,
    },
    PlayFile {
        path: PathBuf,
    },
    PlayUrl {
        url: String,
    },
    Pause,
    Next,
    Prev,
    Status,
    Queue {
        #[command(subcommand)]
        action: QueueAction,
    },
}

#[derive(Subcommand)]
pub enum QueueAction {
    Add { item: String },
    List,
    Next,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Create adapter and player
    let adapter = crate::playback::get_adapter().await?;
    let mut player = Player::new(adapter);

    if cli.daemon {
        // run the simple daemon that listens for JSON commands
        crate::daemon::run_daemon(player).await?;
        return Ok(());
    }

    match cli.command {
        Commands::Search { query } => {
            let res = player
                .adapter_mut()
                .search(&query)
                .await
                .context("search failed")?;
            println!("Search results:\n{}", res);
        }
        Commands::Play { track_id } => {
            player
                .adapter_mut()
                .play(track_id.as_deref())
                .await
                .context("play failed")?;
            println!("Play command sent");
        }
        Commands::PlayFile { path } => {
            let s = path.to_string_lossy().to_string();
            player.play_item(&s).await.context("play file failed")?;
            println!("Playing file: {}", s);
        }
        Commands::PlayUrl { url } => {
            player.play_item(&url).await.context("play url failed")?;
            println!("Playing url: {}", url);
        }
        Commands::Pause => {
            player.adapter_mut().pause().await.context("pause failed")?;
            println!("Paused");
        }
        Commands::Next => {
            player.adapter_mut().next().await.context("next failed")?;
            println!("Next");
        }
        Commands::Prev => {
            player.adapter_mut().prev().await.context("prev failed")?;
            println!("Prev");
        }
        Commands::Status => {
            let s = player
                .adapter_mut()
                .status()
                .await
                .context("status failed")?;
            println!("Status:\n{}", s);
        }
        Commands::Queue { action } => match action {
            QueueAction::Add { item } => {
                player.enqueue(item);
                println!("Queued");
            }
            QueueAction::List => {
                for (i, it) in player.list().iter().enumerate() {
                    println!("{}: {}", i + 1, it);
                }
            }
            QueueAction::Next => {
                if let Some(it) = player.next_item() {
                    player
                        .play_item(&it)
                        .await
                        .context("play queued item failed")?;
                    println!("Playing queued item: {}", it);
                } else {
                    println!("Queue empty");
                }
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parse_play_file() {
        let cli = Cli::parse_from(["apple", "play-file", "song.mp3"]);
        match cli.command {
            Commands::PlayFile { path } => assert_eq!(path, PathBuf::from("song.mp3")),
            _ => panic!("expected PlayFile command"),
        }
    }

    #[test]
    fn parse_queue_add() {
        let cli = Cli::parse_from(["apple", "queue", "add", "http://example.com/stream.mp3"]);
        match cli.command {
            Commands::Queue { action } => match action {
                QueueAction::Add { item } => {
                    assert_eq!(item, "http://example.com/stream.mp3".to_string())
                }
                _ => panic!("expected Queue Add"),
            },
            _ => panic!("expected Queue command"),
        }
    }
}
