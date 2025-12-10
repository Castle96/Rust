Contributing to apple

Thanks for wanting to contribute! A few guidelines to make contributions smooth:

- Code style: run `cargo fmt` and ensure `cargo clippy` reports no warnings.
- Tests: Add unit tests under `src/` and integration tests under `tests/`.
- Integration tests that require external binaries (e.g., `mpv`) should be marked `#[ignore]` and run locally via `cargo test -- --ignored --nocapture`.
- When filing issues or PRs include a short reproduction and the `rustc` and `cargo` versions (run `rustup show`).

Local development commands

- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features`
- Unit tests: `cargo test`
- Integration tests (requires mpv): `cargo test -- --ignored --nocapture`

If you're unsure about a change, open an issue or a draft PR first.

