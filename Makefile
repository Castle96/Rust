# Makefile for developer convenience

.PHONY: fmt clippy test integration clean

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets --all-features

test:
	cargo test

integration:
	cargo test -- --ignored --nocapture

clean:
	cargo clean

