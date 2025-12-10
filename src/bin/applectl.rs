use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use serde::Deserialize;

fn is_insecure_http(s: &str) -> bool {
    s.starts_with("http://")
}

fn insecure_allowed() -> bool {
    std::env::var("APPLE_ALLOW_INSECURE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

#[derive(Parser)]
#[command(name = "applectl")]
struct Cli {
    /// Daemon socket (overrides APPLE_DAEMON_SOCKET)
    #[arg(long)]
    socket: Option<String>,

    /// Auth token (overrides APPLE_DAEMON_TOKEN)
    #[arg(long)]
    token: Option<String>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Play { uri: String },
    Pause,
    Enqueue { uri: String },
    Next,
    Status,
    List,
    ArtistInfo { artist_id: String },
    ArtistDiscography { artist_id: String },
}

#[derive(Deserialize)]
struct Resp {
    ok: bool,
    msg: String,
    items: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let socket = cli.socket.or_else(|| std::env::var("APPLE_DAEMON_SOCKET").ok()).expect("daemon socket required (set APPLE_DAEMON_SOCKET or --socket)");
    let token = cli.token.or_else(|| std::env::var("APPLE_DAEMON_TOKEN").ok());

    match cli.cmd {
        Commands::Play { uri } => {
            if is_insecure_http(&uri) && !insecure_allowed() {
                bail!("Refusing insecure http URL. Use https:// or set APPLE_ALLOW_INSECURE=1 to allow insecure URLs");
            }
            let r = send(&socket, token.as_deref(), "play", Some(&uri)).await?; println!("{}", r.msg);
        }
        Commands::Pause => { let r = send(&socket, token.as_deref(), "pause", None).await?; println!("{}", r.msg); }
        Commands::Enqueue { uri } => {
            if is_insecure_http(&uri) && !insecure_allowed() {
                bail!("Refusing insecure http URL. Use https:// or set APPLE_ALLOW_INSECURE=1 to allow insecure URLs");
            }
            let r = send(&socket, token.as_deref(), "enqueue", Some(&uri)).await?; println!("{}", r.msg);
        }
        Commands::Next => { let r = send(&socket, token.as_deref(), "next", None).await?; println!("{}", r.msg); }
        Commands::Status => { let r = send(&socket, token.as_deref(), "status", None).await?; println!("{}", r.msg); }
        Commands::List => { let r = send(&socket, token.as_deref(), "list", None).await?; if let Some(items) = r.items { for it in items { println!("- {}", it); } } else { println!("no items"); } }
        Commands::ArtistInfo { artist_id } => {
            let r = send(&socket, token.as_deref(), "artist_info", Some(&artist_id)).await?;
            if let Some(items) = r.items { for it in items { println!("{}", it); } } else { println!("{}", r.msg); }
        }
        Commands::ArtistDiscography { artist_id } => {
            let r = send(&socket, token.as_deref(), "artist_discography", Some(&artist_id)).await?;
            if let Some(items) = r.items { for it in items { println!("- {}", it); } } else { println!("{}", r.msg); }
        }
    }

    Ok(())
}

async fn send(socket: &str, token: Option<&str>, cmd: &str, arg: Option<&str>) -> Result<Resp> {
    #[cfg(unix)]
    {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, split};
        use tokio::net::UnixStream;
        let stream = UnixStream::connect(socket).await?;
        let (r, mut w) = split(stream);
        let mut reader = BufReader::new(r);
        let payload = serde_json::json!({"cmd": cmd, "arg": arg, "token": token});
        let msg = payload.to_string() + "\n";
        w.write_all(msg.as_bytes()).await?;
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let resp: Resp = serde_json::from_str(&line)?;
        Ok(resp)
    }
    #[cfg(not(unix))]
    {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, split};
        use tokio::net::TcpStream;
        let stream = TcpStream::connect(socket).await?;
        let (r, mut w) = split(stream);
        let mut reader = BufReader::new(r);
        let payload = serde_json::json!({"cmd": cmd, "arg": arg, "token": token});
        let msg = payload.to_string() + "\n";
        w.write_all(msg.as_bytes()).await?;
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let resp: Resp = serde_json::from_str(&line)?;
        Ok(resp)
    }
}
