//! Integration tests that exercise real playback adapters.
//!
//! These tests are ignored by default because they spawn external processes (python http.server and mpv).

use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use tokio::time::sleep;

use apple::playback::PlaybackAdapter;

// Only compile/run on unix (mpv adapter is provided under cfg(unix)).
#[cfg(unix)]
#[tokio::test]
#[ignore]
async fn plays_silence_via_mpv() {
    // ensure mpv is available
    if which::which("mpv").is_err() {
        eprintln!("mpv not found in PATH; skipping integration test");
        return;
    }

    // Create a temporary directory for the fixture
    let mut dir = std::env::temp_dir();
    let uniq = format!(
        "apple-integration-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    dir.push(uniq);
    std::fs::create_dir_all(&dir).expect("create temp dir");

    // Write a 1 second silent WAV (16-bit PCM, mono, 44100 Hz)
    let path = dir.join("silence.wav");
    write_silence_wav(&path).expect("write wav");

    // Bind a TcpListener on port 0 to get a free port and keep the listener
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind listener");
    let port = listener.local_addr().expect("local addr").port();

    let listener = Arc::new(listener);
    let server_listener = listener.try_clone().expect("clone listener");

    // Shared shutdown flag
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_thread = shutdown.clone();

    // Spawn server thread to accept connections and serve the WAV file
    let path_for_thread = path.clone();
    let server_handle = thread::spawn(move || {
        for stream in server_listener.incoming() {
            if shutdown_thread.load(Ordering::SeqCst) {
                break;
            }
            match stream {
                Ok(mut s) => {
                    // Read request (not used)
                    let mut buf = [0u8; 1024];
                    let _ = s.read(&mut buf);

                    // Read file
                    let mut f = File::open(&path_for_thread).expect("open wav");
                    let mut data = Vec::new();
                    f.read_to_end(&mut data).expect("read wav");

                    // Write a minimal HTTP response
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: audio/wav\r\nConnection: close\r\n\r\n",
                        data.len()
                    );
                    let _ = s.write_all(header.as_bytes());
                    let _ = s.write_all(&data);
                    let _ = s.flush();
                }
                Err(_) => break,
            }
        }
    });

    // RAII guard that ensures server shutdown, thread join, and temp-file cleanup
    struct TestServerGuard {
        shutdown: Arc<AtomicBool>,
        handle: Option<thread::JoinHandle<()>>,
        server_addr: String,
        path: PathBuf,
        dir: PathBuf,
    }

    impl Drop for TestServerGuard {
        fn drop(&mut self) {
            // signal shutdown
            self.shutdown.store(true, Ordering::SeqCst);
            // wake accept
            let _ = TcpStream::connect(&self.server_addr);
            // join thread
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
            // cleanup files
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_dir(&self.dir);
        }
    }

    // Create server address and guard now so we can use guard on readiness failure
    let server_addr = format!("127.0.0.1:{}", port);
    let guard = TestServerGuard {
        shutdown: shutdown.clone(),
        handle: Some(server_handle),
        server_addr: server_addr.clone(),
        path: path.clone(),
        dir: dir.clone(),
    };

    // Wait for server readiness (retry connecting to the port)
    let mut ready = false;
    for _ in 0..50 {
        if TcpStream::connect(&server_addr).is_ok() {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if !ready {
        // cleanup via guard and panic
        drop(guard);
        panic!("http server did not become ready on {}", server_addr);
    }

    // URL to the WAV file
    let url = format!(
        "http://127.0.0.1:{}/{}",
        port,
        path.file_name().unwrap().to_string_lossy()
    );

    // Create an MpvAdapter and play the URL
    let mut adapter = apple::playback::MpvAdapter::try_new()
        .await
        .expect("failed to start mpv");

    adapter
        .play(Some(&url))
        .await
        .expect("mpv failed to play url");

    // Let it play for a short while then pause
    sleep(Duration::from_millis(1200)).await;

    adapter.pause().await.expect("failed to pause");

    // Explicitly drop the guard to perform cleanup before exiting the test
    drop(guard);
}

fn write_silence_wav(path: &PathBuf) -> std::io::Result<()> {
    let sample_rate: u32 = 44100;
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let duration_secs: u32 = 1;
    let num_samples = sample_rate * duration_secs;

    let byte_rate = sample_rate * (bits_per_sample as u32) / 8 * num_channels as u32;
    let block_align = num_channels * (bits_per_sample / 8);
    let subchunk2_size = num_samples * (bits_per_sample as u32 / 8) * num_channels as u32;
    let chunk_size = 36 + subchunk2_size;

    let mut f = File::create(path)?;

    // RIFF header
    f.write_all(b"RIFF")?;
    f.write_all(&chunk_size.to_le_bytes())?;
    f.write_all(b"WAVE")?;

    // fmt subchunk
    f.write_all(b"fmt ")?;
    f.write_all(&16u32.to_le_bytes())?; // Subchunk1Size for PCM
    f.write_all(&1u16.to_le_bytes())?; // AudioFormat PCM = 1
    f.write_all(&num_channels.to_le_bytes())?;
    f.write_all(&sample_rate.to_le_bytes())?;
    f.write_all(&byte_rate.to_le_bytes())?;
    f.write_all(&block_align.to_le_bytes())?;
    f.write_all(&bits_per_sample.to_le_bytes())?;

    // data subchunk
    f.write_all(b"data")?;
    f.write_all(&subchunk2_size.to_le_bytes())?;

    // write samples (silence)
    for _ in 0..num_samples {
        f.write_all(&0i16.to_le_bytes())?;
    }

    Ok(())
}
