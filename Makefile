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

.PHONY: godot-build-win
godot-build-win:
	rustup target add x86_64-pc-windows-gnu || true
	cargo build -p tiletangle-godot --release --target x86_64-pc-windows-gnu
	@echo "Output: target/x86_64-pc-windows-gnu/release/tiletangle_godot.dll"

.PHONY: godot-build-macos
godot-build-macos:
	@echo "Building on macOS runner recommended (GitHub Actions)."
	@echo "Use: cargo build -p tiletangle-godot --release (on macOS)"

.PHONY: godot-copy-libs
godot-copy-libs:
	mkdir -p bindings/godot/examples/bin/linux bindings/godot/examples/bin/windows bindings/godot/examples/bin/macos
	@if [ -f target/release/libtiletangle_godot.so ]; then cp -f target/release/libtiletangle_godot.so bindings/godot/examples/bin/linux/; fi
	@if [ -f target/x86_64-pc-windows-gnu/release/tiletangle_godot.dll ]; then cp -f target/x86_64-pc-windows-gnu/release/tiletangle_godot.dll bindings/godot/examples/bin/windows/; fi
	@if [ -f target/release/libtiletangle_godot.dylib ]; then cp -f target/release/libtiletangle_godot.dylib bindings/godot/examples/bin/macos/; fi

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

WEB_PROJ:=bindings/godot/examples
WEB_DIR:=$(WEB_PROJ)/dist/web

.PHONY: web-export
web-export:
	mkdir -p $(WEB_DIR)
	tools/godot-4.5/godot4 --headless --path $(WEB_PROJ) --export-release "Web" dist/web/index.html

.PHONY: web-wire
web-wire:
	@echo "Copying WASM package and shim into $(WEB_DIR)..."
	cp -f wasm/pkg/tiletangle_wasm.js wasm/pkg/tiletangle_wasm_bg.wasm $(WEB_DIR)/
	cp -f tools/web/web_engine.js $(WEB_DIR)/
	@echo "Copying dictionary assets..."
	mkdir -p $(WEB_DIR)/dictionaries
	@if [ -f docs/static/dictionaries/TWL06.gaddag.cbor.gz ]; then cp -f docs/static/dictionaries/TWL06.gaddag.cbor.gz $(WEB_DIR)/dictionaries/; fi
	@if [ -f docs/static/dictionaries/TWL06.fst ]; then cp -f docs/static/dictionaries/TWL06.fst $(WEB_DIR)/dictionaries/; fi
	@if [ -f docs/static/dictionaries/TWL06.txt ]; then cp -f docs/static/dictionaries/TWL06.txt $(WEB_DIR)/dictionaries/; fi
	@echo "Injecting shim import into index.html"
	@python3 -c "p='$$(pwd)/$(WEB_DIR)/index.html';import io,sys;html=open(p,'r',encoding='utf-8').read();inj='\n  <script type=\\\"module\\\" src=\\\"web_engine.js\\\"></script>\n';print('Already patched' if inj in html else 'Patching');open(p,'w',encoding='utf-8').write(html if inj in html else html.replace('</head>', inj+'</head>'))"

.PHONY: docs-godot one-shot
# One-shot pipeline: build WASM, export Godot Web, wire shim, publish into Docusaurus
docs-godot: wasm web-export web-wire
	rm -rf docs/static/playground/*
	cp -r $(WEB_DIR)/* docs/static/playground/
	@echo "Published Godot Web build to docs/static/playground"

one-shot: docs-godot

.PHONY: ffi-header
ffi-header:
	cargo install cbindgen --locked || true
	cd engine_ffi && mkdir -p include && cbindgen --config cbindgen.toml --crate tiletangle-engine-ffi --output include/engine.h

.PHONY: python-dev
python-dev:
	python3 -m pip install --upgrade maturin pytest || true
	cd bindings/python && maturin develop --release

.PHONY: install-hooks
install-hooks:
	@# Ensure pre-commit is available for the active Python without relying on shell shims
	@python3 -c 'import pre_commit' >/dev/null 2>&1 || { \
		echo 'Installing pre-commit (user site-packages)...'; \
		python3 -m pip install --user pre-commit; \
	}
	@# Prefer module invocation to bypass problematic shims (pyenv/WSL)
	python3 -m pre_commit install || pre-commit install
