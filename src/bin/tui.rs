// Full-featured TUI for apple
// - Shows status and queue
// - Supports local (in-process) control or remote control via daemon socket (APPLE_DAEMON_SOCKET)
// - Keybindings: q=quit, p=pause, SPACE=toggle pause (pause only), n=play next queued item, s=refresh status
//   a=play immediately (enter input), e=enqueue (enter input), Up/Down navigate queue

use anyhow::Result;
use crossterm::event::{self, Event as CEvent, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::style::{Color, Modifier, Style};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph, Gauge},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::time::{Duration, Instant};

use apple::config::{load_config, save_config};
use apple::player::Player;

#[derive(Deserialize, Serialize, Debug)]
struct DaemonResp {
    ok: bool,
    msg: String,
    items: Option<Vec<String>>,
}

enum Controller {
    Local {
        player: Player,
    },
    Remote {
        socket: String,
        token: Option<String>,
    },
}

impl Controller {
    async fn status(&mut self) -> Result<String> {
        match self {
            Controller::Local { player } => player.adapter_mut().status().await,
            Controller::Remote { socket, token } => {
                let resp = send_daemon_cmd(socket, token.as_deref(), "status", None).await?;
                Ok(resp.msg)
            }
        }
    }

    async fn get_position(&mut self) -> Result<u64> {
        match self {
            Controller::Local { player } => player.adapter_mut().get_position().await,
            Controller::Remote { socket, token } => {
                let resp = send_daemon_cmd(socket, token.as_deref(), "position", None).await?;
                resp.msg.parse().unwrap_or(Ok(0))
            }
        }
    }

    async fn get_duration(&mut self) -> Result<u64> {
        match self {
            Controller::Local { player } => player.adapter_mut().get_duration().await,
            Controller::Remote { socket, token } => {
                let resp = send_daemon_cmd(socket, token.as_deref(), "duration", None).await?;
                resp.msg.parse().unwrap_or(Ok(0))
            }
        }
    }

    async fn volume_up(&mut self) -> Result<()> {
        match self {
            Controller::Local { player } => player.adapter_mut().volume_up().await,
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "volume_up", None).await?;
                Ok(())
            }
        }
    }

    async fn volume_down(&mut self) -> Result<()> {
        match self {
            Controller::Local { player } => player.adapter_mut().volume_down().await,
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "volume_down", None).await?;
                Ok(())
            }
        }
    }

    async fn seek_forward(&mut self) -> Result<()> {
        match self {
            Controller::Local { player } => player.adapter_mut().seek_forward(10).await,
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "seek_forward", Some("10")).await?;
                Ok(())
            }
        }
    }

    async fn seek_backward(&mut self) -> Result<()> {
        match self {
            Controller::Local { player } => player.adapter_mut().seek_backward(10).await,
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "seek_backward", Some("10")).await?;
                Ok(())
            }
        }
    }

    async fn pause(&mut self) -> Result<()> {
        match self {
            Controller::Local { player } => player.adapter_mut().pause().await,
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "pause", None).await?;
                Ok(())
            }
        }
    }

    async fn play_item(&mut self, item: &str) -> Result<()> {
        match self {
            Controller::Local { player } => player.play_item(item).await,
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "play", Some(item)).await?;
                Ok(())
            }
        }
    }

    async fn enqueue(&mut self, item: &str) -> Result<()> {
        match self {
            Controller::Local { player } => {
                player.enqueue(item.to_string());
                Ok(())
            }
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "enqueue", Some(item)).await?;
                Ok(())
            }
        }
    }

    async fn next_and_play(&mut self) -> Result<()> {
        match self {
            Controller::Local { player } => {
                if let Some(it) = player.next_item() {
                    player.play_item(&it).await?;
                }
                Ok(())
            }
            Controller::Remote { socket, token } => {
                let _ = send_daemon_cmd(socket, token.as_deref(), "next", None).await?;
                Ok(())
            }
        }
    }

    async fn list_queue(&mut self) -> Result<Vec<String>> {
        match self {
            Controller::Local { player } => Ok(player.list()),
            Controller::Remote { socket, token } => {
                let resp = send_daemon_cmd(socket, token.as_deref(), "list", None).await?;
                Ok(resp.items.unwrap_or_default())
            }
        }
    }

    async fn artist_info(&mut self, id: &str) -> Result<String> {
        match self {
            Controller::Local { player } => player.adapter_mut().artist_info(id).await,
            Controller::Remote { socket, token } => {
                let resp =
                    send_daemon_cmd(socket, token.as_deref(), "artist_info", Some(id)).await?;
                if let Some(items) = resp.items {
                    Ok(items.join("\n"))
                } else {
                    Ok(resp.msg)
                }
            }
        }
    }

    async fn artist_discography(&mut self, id: &str) -> Result<String> {
        match self {
            Controller::Local { player } => player.adapter_mut().artist_discography(id).await,
            Controller::Remote { socket, token } => {
                let resp =
                    send_daemon_cmd(socket, token.as_deref(), "artist_discography", Some(id))
                        .await?;
                if let Some(items) = resp.items {
                    Ok(items.join("\n"))
                } else {
                    Ok(resp.msg)
                }
            }
        }
    }
}

async fn send_daemon_cmd(
    socket: &str,
    token: Option<&str>,
    cmd: &str,
    arg: Option<&str>,
) -> Result<DaemonResp> {
    #[cfg(unix)]
    {
        use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;
        let stream = UnixStream::connect(socket).await?;
        let (r, mut w) = split(stream);
        let mut reader = BufReader::new(r);
        let payload = serde_json::json!({"cmd": cmd, "arg": arg, "token": token});
        let msg = payload.to_string() + "\n";
        w.write_all(msg.as_bytes()).await?;
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let resp: DaemonResp = serde_json::from_str(&line)?;
        Ok(resp)
    }
    #[cfg(not(unix))]
    {
        use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpStream;
        let stream = TcpStream::connect(socket).await?;
        let (r, mut w) = split(stream);
        let mut reader = BufReader::new(r);
        let payload = serde_json::json!({"cmd": cmd, "arg": arg, "token": token});
        let msg = payload.to_string() + "\n";
        w.write_all(msg.as_bytes()).await?;
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let resp: DaemonResp = serde_json::from_str(&line)?;
        Ok(resp)
    }
}

fn is_insecure_http(s: &str) -> bool {
    s.starts_with("http://")
}
fn insecure_allowed() -> bool {
    std::env::var("APPLE_ALLOW_INSECURE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn format_time(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{:02}:{:02}", mins, secs)
}

#[derive(Clone, Copy, Debug)]
enum Theme {
    Dark,
    Light,
}

impl Theme {
    fn from_env() -> Self {
        match std::env::var("APPLE_THEME").ok().as_deref() {
            Some("light") => Theme::Light,
            _ => Theme::Dark,
        }
    }
    fn next(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }
    fn header_style(self) -> Style {
        match self {
            Theme::Dark => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Theme::Light => Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        }
    }
    fn help_style(self) -> Style {
        match self {
            Theme::Dark => Style::default().fg(Color::Gray),
            Theme::Light => Style::default().fg(Color::DarkGray),
        }
    }
    fn list_highlight(self) -> Style {
        match self {
            Theme::Dark => Style::default()
                .bg(Color::Blue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            Theme::Light => Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        }
    }
    fn modal_style(self) -> Style {
        match self {
            Theme::Dark => Style::default().fg(Color::White),
            Theme::Light => Style::default().fg(Color::Black),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // controller selection
    let mut controller = if let Ok(sock) = std::env::var("APPLE_DAEMON_SOCKET") {
        let token = std::env::var("APPLE_DAEMON_TOKEN").ok();
        Controller::Remote {
            socket: sock,
            token,
        }
    } else {
        let adapter = apple::playback::get_adapter().await?;
        let player = Player::new(adapter);
        Controller::Local { player }
    };

    // UI state
    let mut last_status = controller
        .status()
        .await
        .unwrap_or_else(|_| "unknown".into());
    let mut last_refresh = Instant::now();
    let mut selected: usize = 0;
    let mut mode_input = false;
    let mut input_buf = String::new();
    let mut input_enqueue = false;
    let mut pending_artist_action: Option<&str> = None;

    let mut modal_open = false;
    let mut modal_lines: Vec<String> = Vec::new();
    let mut modal_scroll: usize = 0;

    let mut prefs_open = false;
    let mut prefs_selected: usize = 0;

    let mut cfg = load_config();
    let mut theme = Theme::from_env();
    if let Some(ref t) = cfg.theme {
        theme = if t == "light" {
            Theme::Light
        } else {
            Theme::Dark
        }
    }

    let mut list_state = ratatui::widgets::ListState::default();
    let tick_rate = Duration::from_millis(100);

loop {
        let queue = controller.list_queue().await.unwrap_or_default();
        if queue.is_empty() { selected = 0 } else if selected >= queue.len() { selected = queue.len()-1 }
        list_state.select(if queue.is_empty() { None } else { Some(selected) });

        // Get position and duration outside of draw to avoid async issues
        let position = controller.get_position().await.unwrap_or(0);
        let duration = controller.get_duration().await.unwrap_or(0);

        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default().direction(Direction::Vertical).margin(1)
                .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(4), Constraint::Length(3)]).split(size);

            let header = Paragraph::new(format!("Apple TUI - q:quit p:pause SPACE:pause n:next s:status a:play e:enqueue t:theme +/-:volume ←/→:seek - last: {}", last_status))
                .style(theme.header_style()).block(Block::default().borders(Borders::ALL).title("Controls"));
            f.render_widget(header, chunks[0]);

            // Progress bar
            let progress = if duration > 0 { position as f32 / duration as f32 } else { 0.0 };
            let progress_text = format!("{} / {}", 
                format_time(position), 
                if duration > 0 { format_time(duration) } else { "--:--".to_string() }
            );
            let progress_gauge = ratatui::widgets::Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(theme.list_highlight())
                .percent((progress * 100.0) as u16)
                .label(progress_text);
            f.render_widget(progress_gauge, chunks[1]);

            let items: Vec<ListItem> = queue.iter().map(|it| ListItem::new(it.clone())).collect();
            let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Queue")).highlight_style(theme.list_highlight());
            f.render_stateful_widget(list, chunks[2], &mut list_state);

            if mode_input {
                let prompt = if input_enqueue { "Enqueue: " } else { "Play: " };
                let p = Paragraph::new(format!("{}{}", prompt, input_buf)).block(Block::default().borders(Borders::ALL).title("Input (Enter to submit, Esc to cancel)"));
                f.render_widget(p, chunks[2]);
            } else {
                let help = Paragraph::new("Navigation: Up/Down to move, e:enqueue, a:play, i:artist info, d:discography, T:preferences, t:theme toggle")
                    .style(theme.help_style()).block(Block::default().borders(Borders::ALL).title("Help"));
                f.render_widget(help, chunks[2]);
            }

            if modal_open {
                use ratatui::layout::Alignment;
                let w = (size.width as f32 * 0.7) as u16;
                let h = (size.height as f32 * 0.7) as u16;
                let x = (size.width.saturating_sub(w)) / 2;
                let y = (size.height.saturating_sub(h)) / 2;
                let area = ratatui::layout::Rect::new(x, y, w, h);
                let max_lines = if h >= 3 { (h-2) as usize } else { 0 };
                let visible = modal_lines.iter().skip(modal_scroll).take(max_lines).cloned().collect::<Vec<_>>().join("\n");
                let p = Paragraph::new(visible).style(theme.modal_style()).block(Block::default().borders(Borders::ALL).title("Artist Details (Esc to close, Up/Down to scroll)")).alignment(Alignment::Left);
                f.render_widget(p, area);
            }

            if prefs_open {
                let w = (size.width as f32 * 0.5) as u16;
                let h = (size.height as f32 * 0.4) as u16;
                let x = (size.width.saturating_sub(w)) / 2;
                let y = (size.height.saturating_sub(h)) / 2;
                let area = ratatui::layout::Rect::new(x, y, w, h);
                let options = [format!("Theme: {}", if let Theme::Light = theme { "Light" } else { "Dark" })];
                let items: Vec<ListItem> = options.iter().map(|s| ListItem::new(s.clone())).collect();
                let mut list = List::new(items).block(Block::default().borders(Borders::ALL).title("Preferences (Up/Down, Enter to toggle, Esc to close)"));
                list = list.highlight_style(theme.list_highlight());
                let mut state = ratatui::widgets::ListState::default(); state.select(Some(prefs_selected));
                f.render_stateful_widget(list, area, &mut state);
            }
        })?;

        // handle input
        if event::poll(tick_rate)? {
            if let CEvent::Key(KeyEvent { code, .. }) = event::read()? {
                if mode_input {
                    match code {
                        KeyCode::Char(c) => input_buf.push(c),
                        KeyCode::Backspace => {
                            input_buf.pop();
                        }
                        KeyCode::Enter => {
                            if let Some(action) = pending_artist_action {
                                if action == "info" {
                                    match controller.artist_info(&input_buf).await {
                                        Ok(info) => {
                                            modal_lines =
                                                info.lines().map(|s| s.to_string()).collect();
                                            modal_scroll = 0;
                                            modal_open = true;
                                        }
                                        Err(_) => {
                                            last_status = "failed to fetch artist info".into();
                                        }
                                    }
                                } else if action == "discography" {
                                    match controller.artist_discography(&input_buf).await {
                                        Ok(disc) => {
                                            modal_lines = if disc.is_empty() {
                                                vec!["<no albums>".into()]
                                            } else {
                                                disc.lines().map(|s| s.to_string()).collect()
                                            };
                                            modal_scroll = 0;
                                            modal_open = true;
                                        }
                                        Err(_) => {
                                            last_status = "failed to fetch discography".into();
                                        }
                                    }
                                }
                            } else if is_insecure_http(&input_buf) && !insecure_allowed() {
                                last_status = "Refused insecure http URL; set APPLE_ALLOW_INSECURE=1 to allow".into();
                            } else if input_enqueue {
                                let _ = controller.enqueue(&input_buf).await;
                            } else {
                                let _ = controller.play_item(&input_buf).await;
                            }
                            input_buf.clear();
                            mode_input = false;
                            pending_artist_action = None;
                        }
                        KeyCode::Esc => {
                            input_buf.clear();
                            mode_input = false;
                        }
                        _ => {}
                    }
                } else if modal_open {
                    match code {
                        KeyCode::Up => {
                            modal_scroll = modal_scroll.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            if modal_scroll + 1 < modal_lines.len() {
                                modal_scroll += 1;
                            }
                        }
                        KeyCode::PageUp => {
                            modal_scroll = modal_scroll.saturating_sub(10);
                        }
                        KeyCode::PageDown => {
                            modal_scroll = modal_scroll
                                .saturating_add(10)
                                .min(modal_lines.len().saturating_sub(1));
                        }
                        KeyCode::Esc => {
                            modal_open = false;
                            modal_lines.clear();
                            modal_scroll = 0;
                        }
                        _ => {}
                    }
                } else if prefs_open {
                    match code {
                        KeyCode::Up => {
                            prefs_selected = prefs_selected.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            prefs_selected = prefs_selected.saturating_add(1).min(0);
                        }
                        KeyCode::Enter => {
                            theme = theme.next();
                            cfg.theme = Some(if let Theme::Light = theme {
                                "light".into()
                            } else {
                                "dark".into()
                            });
                            let _ = save_config(&cfg);
                        }
                        KeyCode::Esc => {
                            prefs_open = false;
                        }
                        _ => {}
                    }
                } else {
                    match code {
                        KeyCode::Char('T') => {
                            prefs_open = true;
                            prefs_selected = 0;
                        }
                        KeyCode::Char('t') => {
                            theme = theme.next();
                            cfg.theme = Some(if let Theme::Light = theme {
                                "light".into()
                            } else {
                                "dark".into()
                            });
                            let _ = save_config(&cfg);
                        }
                        KeyCode::Char('q') => break,
                        KeyCode::Char('p') | KeyCode::Char(' ') => {
                            let _ = controller.pause().await;
                            last_status = controller
                                .status()
                                .await
                                .unwrap_or_else(|_| "unknown".into());
                        }
                        KeyCode::Char('s') => {
                            last_status = controller
                                .status()
                                .await
                                .unwrap_or_else(|_| "unknown".into());
                        }
                        KeyCode::Char('n') => {
                            let _ = controller.next_and_play().await;
                            last_status = controller
                                .status()
                                .await
                                .unwrap_or_else(|_| "unknown".into());
                        }
                        KeyCode::Char('a') => {
                            mode_input = true;
                            input_enqueue = false;
                            pending_artist_action = None;
                            input_buf.clear();
                        }
                        KeyCode::Char('e') => {
                            mode_input = true;
                            input_enqueue = true;
                            pending_artist_action = None;
                            input_buf.clear();
                        }
                        KeyCode::Char('i') => {
                            mode_input = true;
                            input_enqueue = false;
                            pending_artist_action = Some("info");
                            input_buf.clear();
                        }
                        KeyCode::Char('d') => {
                            mode_input = true;
                            input_enqueue = false;
                            pending_artist_action = Some("discography");
                            input_buf.clear();
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            let _ = controller.volume_up().await;
                            last_status = "volume up".into();
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            let _ = controller.volume_down().await;
                            last_status = "volume down".into();
                        }
                        KeyCode::Left => {
                            let _ = controller.seek_backward().await;
                            last_status = "seek backward".into();
                        }
                        KeyCode::Right => {
                            let _ = controller.seek_forward().await;
                            last_status = "seek forward".into();
                        }
                        KeyCode::Up => {
                            selected = selected.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            if selected + 1 < queue.len() {
                                selected += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if last_refresh.elapsed() > Duration::from_secs(2) {
            last_status = controller
                .status()
                .await
                .unwrap_or_else(|_| "unknown".into());
            last_refresh = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
