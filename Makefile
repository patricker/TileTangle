.PHONY: build test fmt lint check clean

build:
	cargo build --workspace

test:
	cargo test --workspace --all-features

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

check: fmt lint test

clean:
	cargo clean
