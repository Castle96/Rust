use crate::player::Player;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

// A tiny JSON command protocol for local control. This is D1: a small daemon mode.
// Commands are sent as a single-line JSON object. Example:
// { "cmd": "play", "arg": "http://...", "token": "optional" }

#[derive(Deserialize)]
struct Cmd {
    cmd: String,
    arg: Option<String>,
    token: Option<String>,
}

#[derive(Serialize)]
struct Resp {
    ok: bool,
    msg: String,
    items: Option<Vec<String>>,
}

/// Run the daemon. Improvements:
/// - Socket path configurable via APPLE_DAEMON_SOCKET
/// - Optional auth token via APPLE_DAEMON_TOKEN
/// - Graceful shutdown on Ctrl-C / SIGTERM
/// - Per-connection loop (multiple commands), per-request timeout
pub async fn run_daemon(player: Player) -> Result<()> {
    let socket_env = std::env::var("APPLE_DAEMON_SOCKET").ok();
    let token_env = std::env::var("APPLE_DAEMON_TOKEN").ok();

    // Shared shutdown notifier
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    // Spawn a task to watch for Ctrl-C (cross-platform) and notify shutdown
    {
        let s = shutdown.clone();
        let flag = shutdown_flag.clone();
        tokio::spawn(async move {
            // On unix we also try to listen for SIGTERM for CI/runners
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                // combine ctrl_c + SIGTERM
                let mut sigterm = signal(SignalKind::terminate()).ok();
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = async {
                        if let Some(sig) = &mut sigterm { sig.recv().await; }
                    } => {}
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            println!("daemon: shutting down (signal received)");
            flag.store(true, Ordering::SeqCst);
            s.notify_waiters();
        });
    }

    #[cfg(unix)]
    {
        use tokio::net::UnixListener;
        let sock = socket_env
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join(format!("apple-daemon-{}.sock", std::process::id())));
        // remove if exists
        let _ = std::fs::remove_file(&sock);
        let listener = UnixListener::bind(&sock)?;
        println!("daemon listening on {}", sock.display());

        // Share player state across tasks
        let player = Arc::new(tokio::sync::Mutex::new(player));
        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                accept = listener.accept() => match accept {
                    Ok((stream, _addr)) => {
                        let player = player.clone();
                        let shutdown = shutdown.clone();
                        let token_env = token_env.clone();
                        let shutdown_flag = shutdown_flag.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_unix_connection(stream, player, shutdown, shutdown_flag, token_env).await {
                                eprintln!("daemon connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("daemon accept error: {}", e);
                        break;
                    }
                }
            }
        }

        // cleanup socket on exit
        let _ = std::fs::remove_file(&sock);
        println!("daemon stopped");
        Ok(())
    }

    #[cfg(not(unix))]
    {
        // Fallback TCP listener bound to localhost:0 (ephemeral port)
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        println!("daemon listening on {}", listener.local_addr()?);
        let player = Arc::new(tokio::sync::Mutex::new(player));
        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                accept = listener.accept() => match accept {
                    Ok((stream, _addr)) => {
                        let player = player.clone();
                        let shutdown = shutdown.clone();
                        let token_env = token_env.clone();
                        let shutdown_flag = shutdown_flag.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_tcp_connection(stream, player, shutdown, shutdown_flag, token_env).await {
                                eprintln!("daemon tcp conn error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("daemon accept error: {}", e);
                        break;
                    }
                }
            }
        }
        println!("daemon stopped");
        Ok(())
    }
}

#[cfg(unix)]
async fn handle_unix_connection(
    stream: tokio::net::UnixStream,
    player: Arc<tokio::sync::Mutex<Player>>,
    _shutdown_notify: Arc<tokio::sync::Notify>,
    shutdown_flag: Arc<AtomicBool>,
    token_env: Option<String>,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, split};
    let (r, mut w) = split(stream);
    let mut reader = BufReader::new(r);
    loop {
        // read a line with timeout
        let mut line = String::new();
        match tokio::time::timeout(Duration::from_secs(30), reader.read_line(&mut line)).await {
            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => {
                // EOF, read error, or timeout: close connection
                break;
            }
            Ok(Ok(_)) => {}
        }

        // allow shutdown to preempt long handling
        if shutdown_flag.load(Ordering::SeqCst) {
            break;
        }

        // parse and handle command
        match serde_json::from_str::<Cmd>(&line) {
            Ok(c) => {
                // token check if required
                if let Some(ref expected) = token_env {
                    if c.token.as_deref() != Some(expected.as_str()) {
                        let resp = Resp {
                            ok: false,
                            msg: "unauthorized".into(),
                            items: None,
                        };
                        let j = serde_json::to_string(&resp)? + "\n";
                        let _ = w.write_all(j.as_bytes()).await;
                        continue;
                    }
                }

                let mut pl = player.lock().await;
                let res = match c.cmd.as_str() {
                    "play" => {
                        if let Some(u) = c.arg.as_deref() {
                            // block insecure http unless explicitly allowed via env
                            if u.starts_with("http://") && !std::env::var("APPLE_ALLOW_INSECURE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false) {
                                Resp { ok: false, msg: "Refusing insecure http URL; set APPLE_ALLOW_INSECURE=1 to allow".into(), items: None }
                            } else if u.starts_with("https://") {
                                if let Err(e) = validate_https_url(u).await {
                                    Resp { ok: false, msg: format!("url validation failed: {}", e), items: None }
                                } else {
                                    let _ = pl.play_item(u).await;
                                    Resp { ok: true, msg: "playing".into(), items: None }
                                }
                            } else {
                                // allow other schemes (file://, etc.) without validation
                                let _ = pl.play_item(u).await;
                                Resp { ok: true, msg: "playing".into(), items: None }
                            }
                        } else { Resp { ok: false, msg: "missing arg".into(), items: None } }
                    }
                    "pause" => {
                        let _ = pl.adapter_mut().pause().await;
                        Resp {
                            ok: true,
                            msg: "paused".into(),
                            items: None,
                        }
                    }
                    "enqueue" => {
                        if let Some(item) = c.arg.as_deref() {
                            if item.starts_with("http://") && !std::env::var("APPLE_ALLOW_INSECURE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false) {
                                Resp { ok: false, msg: "Refusing insecure http URL; set APPLE_ALLOW_INSECURE=1 to allow".into(), items: None }
                            } else if item.starts_with("https://") {
                                if let Err(e) = validate_https_url(item).await {
                                    Resp { ok: false, msg: format!("url validation failed: {}", e), items: None }
                                } else {
                                    pl.enqueue(item.to_string());
                                    Resp { ok: true, msg: "enqueued".into(), items: None }
                                }
                            } else {
                                pl.enqueue(item.to_string());
                                Resp { ok: true, msg: "enqueued".into(), items: None }
                            }
                        } else {
                            Resp { ok: false, msg: "missing arg".into(), items: None }
                        }
                    }
                    "next" => {
                        if let Some(it) = pl.next_item() {
                            let _ = pl.play_item(&it).await;
                            Resp { ok: true, msg: format!("playing {}", it), items: None }
                        } else {
                            Resp { ok: false, msg: "queue empty".into(), items: None }
                        }
                    }
                    "status" => {
                        let s = pl
                            .adapter_mut()
                            .status()
                            .await
                            .unwrap_or_else(|e| format!("err: {}", e));
                        Resp { ok: true, msg: s, items: None }
                    }
                    "list" => Resp { ok: true, msg: "ok".into(), items: Some(pl.list()) },
                    "artist_info" => {
                        if let Some(artist_id) = c.arg.as_deref() {
                            let info = pl.adapter_mut().artist_info(artist_id).await.unwrap_or_else(|e| format!("err: {}", e));
                            // split lines into items for structured response
                            let items = info.lines().map(|s| s.to_string()).collect();
                            Resp { ok: true, msg: "artist info".into(), items: Some(items) }
                        } else {
                            Resp { ok: false, msg: "missing arg".into(), items: None }
                        }
                    }
                    "artist_discography" => {
                        if let Some(artist_id) = c.arg.as_deref() {
                            let disc = pl.adapter_mut().artist_discography(artist_id).await.unwrap_or_else(|e| format!("err: {}", e));
                            let items = if disc.is_empty() { vec![] } else { disc.lines().map(|s| s.to_string()).collect() };
                            Resp { ok: true, msg: "discography".into(), items: Some(items) }
                        } else {
                            Resp { ok: false, msg: "missing arg".into(), items: None }
                        }
                    }
                    _ => Resp {
                        ok: false,
                        msg: "unknown cmd".into(),
                        items: None,
                    },
                };
                let j = serde_json::to_string(&res)? + "\n";
                let _ = w.write_all(j.as_bytes()).await;
            }
            Err(_) => {
                let _ = w
                    .write_all(b"{\"ok\":false,\"msg\":\"parse error\"}\n")
                    .await;
            }
        }
    }
    Ok(())
}

#[cfg(not(unix))]
async fn handle_tcp_connection(
    mut stream: tokio::net::TcpStream,
    player: Arc<tokio::sync::Mutex<Player>>,
    _shutdown_notify: Arc<tokio::sync::Notify>,
    shutdown_flag: Arc<AtomicBool>,
    token_env: Option<String>,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, split};
    let (r, mut w) = split(stream);
    let mut reader = BufReader::new(r);
    loop {
        // read a line with timeout
        let mut line = String::new();
        match tokio::time::timeout(Duration::from_secs(30), reader.read_line(&mut line)).await {
            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => {
                // EOF, read error, or timeout: close connection
                break;
            }
            Ok(Ok(_)) => {}
        }

        if shutdown_flag.load(Ordering::SeqCst) { break; }

        match serde_json::from_str::<Cmd>(&line) {
            Ok(c) => {
                if let Some(ref expected) = token_env {
                    if c.token.as_deref() != Some(expected.as_str()) {
                        let resp = Resp {
                            ok: false,
                            msg: "unauthorized".into(),
                            items: None,
                        };
                        let j = serde_json::to_string(&resp)? + "\n";
                        let _ = w.write_all(j.as_bytes()).await;
                        continue;
                    }
                }

                let mut pl = player.lock().await;
                let res = match c.cmd.as_str() {
                    "play" => {
                        if let Some(u) = c.arg.as_deref() {
                            // block insecure http unless explicitly allowed via env
                            if u.starts_with("http://") && !std::env::var("APPLE_ALLOW_INSECURE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false) {
                                Resp { ok: false, msg: "Refusing insecure http URL; set APPLE_ALLOW_INSECURE=1 to allow".into(), items: None }
                            } else if u.starts_with("https://") {
                                if let Err(e) = validate_https_url(u).await {
                                    Resp { ok: false, msg: format!("url validation failed: {}", e), items: None }
                                } else {
                                    let _ = pl.play_item(u).await;
                                    Resp { ok: true, msg: "playing".into(), items: None }
                                }
                            } else {
                                // allow other schemes (file://, etc.) without validation
                                let _ = pl.play_item(u).await;
                                Resp { ok: true, msg: "playing".into(), items: None }
                            }
                        } else { Resp { ok: false, msg: "missing arg".into(), items: None } }
                    }
                    "pause" => {
                        let _ = pl.adapter_mut().pause().await;
                        Resp {
                            ok: true,
                            msg: "paused".into(),
                            items: None,
                        }
                    }
                    "enqueue" => {
                        if let Some(item) = c.arg.as_deref() {
                            if item.starts_with("http://") && !std::env::var("APPLE_ALLOW_INSECURE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false) {
                                Resp { ok: false, msg: "Refusing insecure http URL; set APPLE_ALLOW_INSECURE=1 to allow".into(), items: None }
                            } else if item.starts_with("https://") {
                                if let Err(e) = validate_https_url(item).await {
                                    Resp { ok: false, msg: format!("url validation failed: {}", e), items: None }
                                } else {
                                    pl.enqueue(item.to_string());
                                    Resp { ok: true, msg: "enqueued".into(), items: None }
                                }
                            } else {
                                pl.enqueue(item.to_string());
                                Resp { ok: true, msg: "enqueued".into(), items: None }
                            }
                        } else {
                            Resp { ok: false, msg: "missing arg".into(), items: None }
                        }
                    }
                    "next" => {
                        if let Some(it) = pl.next_item() {
                            let _ = pl.play_item(&it).await;
                            Resp { ok: true, msg: format!("playing {}", it), items: None }
                        } else {
                            Resp { ok: false, msg: "queue empty".into(), items: None }
                        }
                    }
                    "status" => {
                        let s = pl
                            .adapter_mut()
                            .status()
                            .await
                            .unwrap_or_else(|e| format!("err: {}", e));
                        Resp { ok: true, msg: s, items: None }
                    }
                    "list" => Resp { ok: true, msg: "ok".into(), items: Some(pl.list()) },
                    "artist_info" => {
                        if let Some(artist_id) = c.arg.as_deref() {
                            let info = pl.adapter_mut().artist_info(artist_id).await.unwrap_or_else(|e| format!("err: {}", e));
                            // split lines into items for structured response
                            let items = info.lines().map(|s| s.to_string()).collect();
                            Resp { ok: true, msg: "artist info".into(), items: Some(items) }
                        } else {
                            Resp { ok: false, msg: "missing arg".into(), items: None }
                        }
                    }
                    "artist_discography" => {
                        if let Some(artist_id) = c.arg.as_deref() {
                            let disc = pl.adapter_mut().artist_discography(artist_id).await.unwrap_or_else(|e| format!("err: {}", e));
                            let items = if disc.is_empty() { vec![] } else { disc.lines().map(|s| s.to_string()).collect() };
                            Resp { ok: true, msg: "discography".into(), items: Some(items) }
                        } else {
                            Resp { ok: false, msg: "missing arg".into(), items: None }
                        }
                    }
                    _ => Resp {
                        ok: false,
                        msg: "unknown cmd".into(),
                        items: None,
                    },
                };
                let j = serde_json::to_string(&res)? + "\n";
                let _ = w.write_all(j.as_bytes()).await;
            }
            Err(_) => {
                let _ = w
                    .write_all(b"{\"ok\":false,\"msg\":\"parse error\"}\n")
                    .await;
            }
        }
    }
    Ok(())
}

async fn validate_https_url(url: &str) -> anyhow::Result<()> {
    // Only validate https URLs
    if !url.starts_with("https://") {
        return Ok(());
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    // Use HEAD first, fall back to GET if HEAD not allowed
    let resp = client.head(url).send().await;
    match resp {
        Ok(r) => {
            if r.status().is_success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("non-success status {}", r.status()))
            }
        }
        Err(_) => {
            // Try GET as many servers don't implement HEAD; only check TLS here
            let r2 = client.get(url).send().await?;
            if r2.status().is_success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("non-success status {}", r2.status()))
            }
        }
    }
}
