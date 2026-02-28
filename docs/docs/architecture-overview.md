---
sidebar_position: 2
---

# Architecture Overview

TileTangle is a deterministic, platform-independent word-game engine written in Rust. A single core crate drives all gameplay logic; thin binding layers expose the engine to WASM/JS, Python, Unity, and Godot consumers through a uniform JSON-based API.

## Workspace layout

| Crate | Package | Purpose |
|---|---|---|
| `engine/` | `tiletangle-engine` | Core game logic (deterministic, no platform deps) |
| `engine_ffi/` | `tiletangle-engine-ffi` | C ABI wrapper (`cbindgen` generates `engine.h`) |
| `wasm/` | `tiletangle-wasm` | `wasm-bindgen` wrapper for browser and Node |
| `bindings/python/` | `tiletangle-python` | PyO3/maturin Python binding |
| `bindings/godot/` | `tiletangle-godot` | GDExtension binding for Godot 4.2+ |
| `tools/dict-build/` | `dict-build` | CLI to compile text wordlists into FST files |

## Engine core (`engine/src/`)

The engine is fully deterministic — identical inputs always produce identical outputs regardless of platform. All randomness flows through a seedable RNG.

### Key modules

- **`game` / `game::state`** — `GameConfig`, `GameState`, `Player`, `MoveDraft`, and the event log. `GameState` owns the board, tile bag, player racks, turn tracking, and seeded RNG.
- **`board`** — `Board`, `Cell`, `Bonus`. The board is generic over its geometry, enabling both standard grids and arbitrary graph topologies.
- **`geometry`** — `BoardGeometry` trait with two implementations: `RectGridGeometry` (standard rectangular grid) and `GraphOverlay` (arbitrary adjacency for hex, 3D layers, or custom shapes). `CellId` is the universal cell index used across the engine.
- **`inventory`** — `Tileset` (tile kind definitions with scores and symbols), `Bag` (tile pool with RNG-driven draws), `Rack` (per-player tile hand).
- **`rules/crossword`** — `CrosswordRules` implements the `Rules` trait: move validation, scoring with word/letter multipliers, bingo bonus, and stacking support.
- **`rules/plugins`** — `RulePlugin` trait, `PluginRules` composite, `BasicActionsPlugin`, `ScoreBonusPlugin`. Provides an extensible rule system for custom game variants.
- **`dict/`** — `Dictionary` trait with four backends: `SetDictionary` (HashSet), `FstDictionary` (finite-state transducer), `DawgDictionary` (DAWG), and `GaddagDictionary` (GADDAG with cursor-based traversal). All support case-folding and Unicode normalization.
- **`movegen`** — `generate_moves` produces a list of `CandidateMove` values from the current board state, dictionary, and rack using an anchor/cross-check algorithm.
- **`ai/`** — `best_move` / `best_move_greedy`, `AiConfig`, `AiDifficulty` (Easy/Medium/Hard), `OpponentModel`, and heuristic evaluation for rack leave, board equity, and endgame adjustments.
- **`text`** — `Tokenizer` (grapheme-cluster-aware segmentation for emoji and multi-character tiles), `NormalizationMode` (NFC/NFD/NFKC/NFKD), `Symbol` type.

### Feature flags

| Flag | Effect |
|---|---|
| `parallel` | Enables Rayon for parallel move generation and AI evaluation |
| `simd` | Enables SIMD-accelerated scoring via the `wide` crate |

## Binding pattern

All bindings follow the same architecture:

1. Accept a JSON config string and deserialize it into internal types.
2. Construct a `GameState` + `CrosswordRules` from the config.
3. Expose methods: `play_move(json)`, `pass_turn()`, `exchange_tiles(json)`, `get_board_json()`, `ai_move()`, `undo`/`redo`, etc.
4. Return results as JSON strings — FFI returns `*mut c_char`, WASM returns `JsValue`, Python returns `str`, Godot returns `GString`.

This JSON-in/JSON-out boundary keeps the bindings thin and the core engine free of serialization concerns beyond `serde`.

## Data flow

```text
┌─────────────┐     JSON config      ┌──────────────┐
│  Host app   │ ───────────────────▶  │   Binding    │
│ (JS/Py/C#/  │                       │  (FFI/WASM/  │
│  GDScript)  │  ◀─────────────────── │  PyO3/gdext) │
└─────────────┘     JSON results      └──────┬───────┘
                                             │
                                    deserialize/serialize
                                             │
                                      ┌──────▼───────┐
                                      │    Engine     │
                                      │  GameState +  │
                                      │  Rules +      │
                                      │  Dictionary   │
                                      └──────────────┘
```

## Determinism guarantees

- The engine uses `StdRng` seeded from `GameConfig::rng_seed`, so tile draws and AI noise are reproducible.
- Board state is captured via Zobrist hashing for efficient position comparison.
- Snapshots (JSON or CBOR) serialize the full game state including RNG position, enabling save/load and replay.

See [Concepts](concepts/boards) for interactive explanations of each subsystem.
