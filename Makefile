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

.PHONY: ffi-header
ffi-header:
	cargo install cbindgen --locked || true
	cd engine_ffi && mkdir -p include && cbindgen --config cbindgen.toml --crate tiletangle-engine-ffi --output include/engine.h

.PHONY: python-dev
python-dev:
	python3 -m pip install --upgrade maturin pytest || true
	cd bindings/python && maturin develop --release
