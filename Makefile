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
	cargo run -q -p dict-build -- assets/dictionaries/TWL06.txt docs/static/dictionaries/TWL06.fst --min-len 2 --case-fold true || true
	cd wasm && wasm-pack build --target web --out-dir pkg
	mkdir -p docs/static/wasm/engine
	rm -rf docs/static/wasm/engine/pkg && cp -r wasm/pkg docs/static/wasm/engine/
	mkdir -p docs/static/dictionaries
	cp -f assets/dictionaries/TWL06.txt docs/static/dictionaries/TWL06.txt

.PHONY: ffi-header
ffi-header:
	cargo install cbindgen --locked || true
	cd engine_ffi && mkdir -p include && cbindgen --config cbindgen.toml --crate tiletangle-engine-ffi --output include/engine.h

.PHONY: python-dev
python-dev:
	python3 -m pip install --upgrade maturin pytest || true
	cd bindings/python && maturin develop --release
