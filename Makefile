.PHONY: build test fmt lint check clean

build:
	cargo build --workspace

test:
	# Lint engine tests strictly before running full workspace tests
	cargo clippy -p tiletangle-engine --tests -- -D warnings
	cargo test --workspace --all-features

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: lint-core
lint-core:
	cargo clippy -p tiletangle-engine -p tiletangle-engine-ffi --all-targets --all-features -- -D warnings

check: fmt lint test

clean:
	cargo clean

.PHONY: unity-libs
unity-libs:
	cargo build -p tiletangle-engine-ffi --release
	@echo "Built shared library at:"
	@echo "  Linux:  target/release/libtiletangle_ffi.so"
	@echo "  macOS:  target/release/libtiletangle_ffi.dylib"
	@echo "  Windows: target/release/tiletangle_ffi.dll"

.PHONY: godot-build
godot-build:
	cargo build -p tiletangle-godot --release

.PHONY: wasm
wasm:
	rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
	which wasm-pack >/dev/null 2>&1 || cargo install wasm-pack --locked
	cargo run -q -p dict-build -- assets/dictionaries/TWL06.txt docs/static/dictionaries/TWL06.fst --min-len 2 --case-fold
	# Also precompute a cached GADDAG next to the text and FST assets
	cargo run -q -p tiletangle-engine --bin gaddag_cache -- docs/static/dictionaries/TWL06.txt docs/static/dictionaries/TWL06.gaddag.cbor.gz --case-fold --norm nfc --gzip
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
