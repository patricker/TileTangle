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

.PHONY: wasm
wasm:
	rustup target add wasm32-unknown-unknown || true
	cargo install wasm-pack --locked || true
	cd wasm && wasm-pack build --target web --out-dir pkg
	mkdir -p docs/static/wasm/engine
	rm -rf docs/static/wasm/engine/pkg && cp -r wasm/pkg docs/static/wasm/engine/
