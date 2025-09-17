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

Packaging & Releasing
---------------------

1. Run `./tools/package_release.py` to build the Rust crate tarball, Python wheel, npm bundle, and
   Unity/Godot archives under `dist/`.
2. Inspect `dist/manifest.json` to confirm the produced artifacts.
3. Publish each artifact to its registry (`cargo publish`, `twine upload`, `npm publish`, attach the
   Unity/Godot zips to the GitHub release) and tag the repo (`git tag v0.2.0`).
