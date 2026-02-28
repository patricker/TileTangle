# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Full check (format + lint + test)
make check

# Build all workspace crates
make build                    # or: cargo build --workspace

# Run tests (lints engine tests with clippy first, then runs full suite)
make test

# Run a single test by name
cargo test -p tiletangle-engine test_name

# Run tests for a specific crate
cargo test -p tiletangle-engine --all-features
cargo test -p tiletangle-engine-ffi

# Format
make fmt                      # or: cargo fmt --all

# Lint (clippy, all workspace targets)
make lint
# Lint core crates only (matches CI)
make lint-core

# Benchmarks (Criterion)
cargo bench -p tiletangle-engine

# WASM build (requires wasm-pack)
make wasm

# Python binding dev build
make python-dev               # or: cd bindings/python && maturin develop --release

# Generate C header via cbindgen
make ffi-header

# Godot GDExtension build
make godot-build

# Fuzz targets (requires nightly + cargo-fuzz)
cargo +nightly fuzz run fuzz_config_load -- -max_total_time=20
cargo +nightly fuzz run fuzz_move_draft -- -max_total_time=20
```

## CI

CI runs on ubuntu/macOS/Windows with stable Rust. Key checks:
- `cargo build/test --workspace --all-features --exclude tiletangle-python` (Python crate tested separately with maturin)
- Clippy strict (`-D warnings`) on core crates only: `tiletangle-engine` and `tiletangle-engine-ffi`
- `cargo fmt --all -- --check`
- WASM smoke test via Node (`wasm/ci_smoke.mjs`)
- C FFI smoke test (`engine_ffi/tests/smoke.c`)
- Python pytest (`bindings/python/tests/`)
- `cargo-deny` (license/advisory), `cargo-udeps` (unused deps, nightly)
- Criterion benchmarks with baseline comparison

## Pre-commit Hooks

Configured in `.pre-commit-config.yaml`: runs `rustfmt` and workspace clippy on commit. Install with `make install-hooks`.

## Architecture

### Workspace Layout

| Crate | Package name | Purpose |
|---|---|---|
| `engine/` | `tiletangle-engine` | Core game logic (lib name: `engine`) |
| `engine_ffi/` | `tiletangle-engine-ffi` | C ABI wrapper (cbindgen → `engine.h`) |
| `wasm/` | `tiletangle-wasm` | wasm-bindgen wrapper for browser/Node |
| `bindings/python/` | `tiletangle-python` | PyO3/maturin Python binding |
| `bindings/godot/` | `tiletangle-godot` | gdext GDExtension binding |
| `tools/dict-build/` | `dict-build` | CLI to compile text wordlists → FST |
| `examples/plugin_template/` | — | Skeleton for custom rule plugins |

### Engine Core (`engine/src/`)

The engine is deterministic and platform-independent. All bindings wrap the same core types via JSON serialization.

Key modules:
- **`game`** / **`game::state`** — `GameConfig`, `GameState`, `Player`, `MoveDraft`, event log. GameState owns the board, bag, player racks, turn tracking, and RNG.
- **`board`** — `Board`, `Cell`, `Bonus`. Board is generic over geometry.
- **`geometry`** — `BoardGeometry` trait, `RectGridGeometry` (standard grid), `GraphOverlay` (arbitrary adjacency). `CellId` is the universal cell index.
- **`inventory`** — `Tileset` (tile kind definitions), `Bag` (tile pool with RNG draw), `Rack`.
- **`rules/crossword`** — `CrosswordRules` implements the `Rules` trait: move validation, scoring (word/letter multipliers, bingo bonus), reading directions.
- **`rules/plugins`** — `RulePlugin` trait, `PluginRules` composite, `BasicActionsPlugin`, `ScoreBonusPlugin`. Extensible rule system.
- **`dict/`** — `Dictionary` trait with four backends: `SetDictionary` (HashSet), `FstDictionary` (finite state transducer), `DawgDictionary` (DAWG), `GaddagDictionary` (GADDAG with cursor-based traversal). All support case-folding and Unicode normalization.
- **`movegen`** — `generate_moves` produces `CandidateMove` list from board state + dictionary + rack using anchor/cross-check algorithm.
- **`ai/`** — `best_move` / `best_move_greedy`, `AiConfig`, `AiDifficulty` (Easy/Medium/Hard), `OpponentModel`, heuristic evaluation.
- **`text`** — `Tokenizer` (grapheme-cluster-aware segmentation), `NormalizationMode` (NFC/NFD/NFKC/NFKD), `Symbol` type.

### Binding Pattern

All bindings (FFI, WASM, Python, Godot) follow the same pattern:
1. Accept JSON config string → deserialize into internal `JsConfig`/similar struct
2. Construct `GameState` + `CrosswordRules` from config
3. Expose methods: `play_move(json) → score`, `get_board_json()`, `ai_move()`, `undo/redo`, etc.
4. Return results as JSON strings (FFI returns `*mut c_char`, WASM returns `JsValue`, Python returns `str`)

### Feature Flags

- `parallel` — enables Rayon for parallel move generation
- `simd` — enables SIMD-accelerated scoring via `wide` crate

## Guidelines from AGENTS.md

- Do not reference trademarked game names; use neutral descriptors ("classic 15×15 crossword-style board", "bingo bonus", "letter multiplier").
- Keep docs self-contained and present-tense. No roadmap or speculative language.
- Keep edits minimal and focused. Prefer small, reviewable diffs.
- Examples must run as written with all dependencies defined inline.
