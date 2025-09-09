TileTangle
=========

Rust-based, universal, configurable word‑game engine with bindings (WASM/JS, Python, C ABI) and a Docusaurus docs site.

- Docs: `./docs`
- Engine crate: `./engine`
- WASM: `./wasm`
- C ABI: `./engine_ffi`
- Python: `./bindings/python`

Wordlists

- We include an example word list at `assets/dictionaries/TWL06.txt`, sourced from https://scrabutility.com/ (citation for provenance).
- A prebuilt FST is generated at `docs/static/dictionaries/TWL06.fst` via `make wasm` for fast loading in the web demo.
- See docs page “Lexicon Integration” for how to load custom dictionaries.

Getting Started

- Build/test: `make check`
- Web (WASM) demo: `make wasm` and open the docs playground
- Python: `cd bindings/python && maturin develop --release`
- C header: `make ffi-header` (generates `engine_ffi/include/engine.h`)
