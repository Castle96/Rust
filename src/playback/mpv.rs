use crate::playback::PlaybackAdapter;
use anyhow::{Context, Result};
#[cfg(unix)]
use nix::sys::signal::kill as nix_kill;
#[cfg(unix)]
use nix::sys::signal::Signal;
#[cfg(unix)]
use nix::unistd::Pid as NixPid;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::{Child, Command};

pub struct MpvAdapter {
    ipc_path: PathBuf,
    _child: Option<Child>,
}

impl MpvAdapter {
    pub async fn try_new() -> Result<Self> {
        // create a temp path for ipc socket using pid and timestamp
        let pid = std::process::id();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis();
        let ipc_name = format!("apple-mpv-{}-{}.sock", pid, now);
        let ipc_path = std::env::temp_dir().join(ipc_name);
        // Diagnostic output: show where mpv will create IPC socket and logs (useful in tests)
        println!("[mpv-adapter] ipc_path = {}", ipc_path.display());
        let mut cmd = Command::new("mpv");
        // avoid loading user config which could influence behavior in CI / tests
        cmd.arg("--no-config");
        cmd.arg("--no-video");
        cmd.arg("--idle");
        // avoid requiring an audio output device in CI / headless environments
        cmd.arg("--ao=null");
        // increase verbosity to capture startup issues
        cmd.arg("--msg-level=all=debug");
        // ensure mpv writes a detailed log to the same directory
        let log_arg = format!(
            "--log-file={}",
            ipc_path.with_extension("log").to_string_lossy()
        );
        cmd.arg(log_arg);
        cmd.arg(format!("--input-ipc-server={}", ipc_path.to_string_lossy()));
        // Create a log file path next to the ipc socket to read back if mpv fails.
        let log_path = ipc_path.with_extension("log");
        // Create an .err file to capture immediate stderr if mpv fails before writing --log-file
        let err_path = ipc_path.with_extension("err");
        println!("[mpv-adapter] log_path = {}", log_path.display());
        println!("[mpv-adapter] err_path = {}", err_path.display());
        let err_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&err_path)
            .context("failed to create mpv err file")?;
        // Let mpv write its --log-file; capture stderr to err_file for immediate diagnostics. Keep stdout null.
        // Build the full command string and run it via sh -c to ensure options like --input-ipc-server= are passed exactly.
        let log_arg_val = ipc_path.with_extension("log").to_string_lossy().to_string();
        let ipc_arg_val = ipc_path.to_string_lossy().to_string();
        let primary_cmd = format!("mpv --no-config --no-video --idle --ao=null --msg-level=all=debug --log-file='{}' --input-ipc-server='{}'", log_arg_val, ipc_arg_val);
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(primary_cmd)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::from(err_file))
            .spawn()
            .context("failed to spawn mpv via shell")?;

        // helper to wait for ipc socket with a child process reference
        async fn wait_for_ipc(
            ipc_path: &PathBuf,
            child: &mut Child,
            log_path: &PathBuf,
            err_path: &PathBuf,
            attempts: usize,
            interval_ms: u64,
        ) -> Result<bool> {
            let mut last_err: Option<std::io::Error> = None;
            for _ in 0..attempts {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // child exited; read log and bail
                        let log_contents = std::fs::read_to_string(log_path)
                            .unwrap_or_else(|_| "<could not read mpv log>".into());
                        let err_contents = std::fs::read_to_string(err_path)
                            .unwrap_or_else(|_| "<could not read mpv err file>".into());
                        anyhow::bail!("mpv process exited early (status={}); mpv --log-file:\n{}\nmpv stderr:\n{}", status, log_contents, err_contents);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        last_err = Some(std::io::Error::other(format!("try_wait failed: {}", e)));
                    }
                }

                if ipc_path.exists() {
                    match UnixStream::connect(ipc_path).await {
                        Ok(_) => return Ok(true),
                        Err(e) => last_err = Some(e),
                    }
                }
                tokio::time::sleep(Duration::from_millis(interval_ms)).await;
            }
            if let Some(e) = last_err {
                Err(e.into())
            } else {
                Ok(false)
            }
        }

        // First attempt: robust flags
        let connected = wait_for_ipc(&ipc_path, &mut child, &log_path, &err_path, 100, 100)
            .await
            .unwrap_or(false);

        if !connected {
            // initial attempt failed; attempt to respawn mpv with minimal flags as a fallback
            let _ = child.kill().await;
            let _ = child.wait().await;

            // try minimal invocation
            let fallback_cmd = format!("mpv --idle --input-ipc-server='{}'", ipc_arg_val);
            let mut child2 = Command::new("sh")
                .arg("-c")
                .arg(fallback_cmd)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .context("failed to spawn fallback mpv via shell")?;
            // wait shorter time for fallback
            let connected2 = wait_for_ipc(&ipc_path, &mut child2, &log_path, &err_path, 30, 100)
                .await
                .unwrap_or(false);
            if connected2 {
                return Ok(Self {
                    ipc_path,
                    _child: Some(child2),
                });
            } else {
                let _ = child2.kill().await;
                let _ = child2.wait().await;
                // read logs
                let log_contents = std::fs::read_to_string(&log_path)
                    .unwrap_or_else(|_| "<could not read mpv log>".into());
                let err_contents = std::fs::read_to_string(&err_path)
                    .unwrap_or_else(|_| "<could not read mpv err file>".into());
                println!("[mpv-adapter] log_path contents:\n{}", log_contents);
                println!("[mpv-adapter] err_path contents:\n{}", err_contents);
                anyhow::bail!(
                    "mpv failed to start (both attempts). mpv --log-file:\n{}\nmpv stderr:\n{}",
                    log_contents,
                    err_contents
                );
            }
        }

        Ok(Self {
            ipc_path,
            _child: Some(child),
        })
    }

    async fn send_command(&self, cmd: serde_json::Value) -> Result<()> {
        let mut stream = UnixStream::connect(&self.ipc_path)
            .await
            .context("failed to connect to mpv ipc")?;
        let s = cmd.to_string() + "\n";
        stream
            .write_all(s.as_bytes())
            .await
            .context("failed to write to mpv ipc")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PlaybackAdapter for MpvAdapter {
    async fn search(&mut self, _query: &str) -> Result<String> {
        Ok("mpv: search not implemented".to_string())
    }

    async fn play(&mut self, track_id: Option<&str>) -> Result<()> {
        if let Some(id) = track_id {
            let cmd = json!({"command": ["loadfile", id, "replace"]});
            self.send_command(cmd).await?;
        }
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        let cmd = json!({"command": ["cycle", "pause"]});
        self.send_command(cmd).await?;
        Ok(())
    }

    async fn next(&mut self) -> Result<()> {
        let cmd = json!({"command": ["playlist-next", "weak"]});
        self.send_command(cmd).await?;
        Ok(())
    }

    async fn prev(&mut self) -> Result<()> {
        let cmd = json!({"command": ["playlist-prev", "weak"]});
        self.send_command(cmd).await?;
        Ok(())
    }

    async fn status(&mut self) -> Result<String> {
        Ok("mpv: status not implemented".to_string())
    }
}

// On Unix, try a graceful SIGTERM via nix, then fallback to kill+reap.
#[cfg(unix)]
impl Drop for MpvAdapter {
    fn drop(&mut self) {
        if let Some(mut child) = self._child.take() {
            if let Some(pid) = child.id() {
                let _ = nix_kill(NixPid::from_raw(pid as i32), Signal::SIGTERM);
            }
            // Best-effort: we signalled the pid above; attempt to reap without awaiting.
            let _ = child.start_kill(); // best-effort immediate kill if still running
        }
    }
}

// Non-Unix fallback: just attempt to kill & reap the child process.
#[cfg(not(unix))]
impl Drop for MpvAdapter {
    fn drop(&mut self) {
        if let Some(mut child) = self._child.take() {
            let _ = child.kill();
            let _ = child.try_wait();
        }
    }
}
