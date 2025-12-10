apple - Terminal music player (prototype)

This repository contains a small terminal music player written in Rust that uses local playback adapters (mpv) and is modeled after a Spotify terminal player project.

Quick start

- Build: cargo build --release --manifest-path apple/Cargo.toml
- Run tests: cargo test --manifest-path apple/Cargo.toml
- Run integration tests (requires mpv): cargo test --manifest-path apple/Cargo.toml -- --ignored --nocapture

TUI

A terminal UI is available as a binary `tui`:

- Run the TUI locally (in-process adapter):

```sh
cargo run --manifest-path apple/Cargo.toml --bin tui
```

- Run the TUI connected to a daemon:

```sh
export APPLE_DAEMON_SOCKET=/tmp/apple-daemon.sock
export APPLE_DAEMON_TOKEN=mytoken   # optional
cargo run --manifest-path apple/Cargo.toml --bin tui
```

Daemon

Start the daemon (in-process adapter):

```sh
cargo run --manifest-path apple/Cargo.toml -- --daemon
```

The daemon listens on a Unix socket (by default under /tmp) and accepts newline-terminated JSON commands: play/pause/status/enqueue/next/list. You can set `APPLE_DAEMON_SOCKET` and `APPLE_DAEMON_TOKEN` to configure socket path and token.

CLI client (`applectl`)

A small control client is included to send commands to the daemon. Example:

```sh
cargo run --manifest-path apple/Cargo.toml --bin applectl -- --socket /tmp/apple-daemon.sock status
cargo run --manifest-path apple/Cargo.toml --bin applectl -- --socket /tmp/apple-daemon.sock enqueue "https://example.com/stream.mp3"
```

Development

- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features`
- Unit tests: `cargo test`
- Integration tests (requires mpv): `cargo test -- --ignored --nocapture`

Notes

- The AppleMusic adapter is a stub that can be implemented later once credentials are available.
- The daemon implements a minimal JSON protocol for local control and testing.
