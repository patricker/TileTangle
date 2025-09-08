Below is a **ready‑to‑drop‑in `TODO.md`** for the repo. It’s structured as phases with **implementation tasks**, **unit tests**, **documentation tasks**, and **demo guidance** (using **Docusaurus + WASM** and bindings). It assumes a **Rust core**, full **FFI surface**, and demos for **Web (WASM)**, **Python**, **Unity**, and **Godot**.

---

# TODO

> **Goal:** Build a **universal, Rust‑based, fully configurable word‑game engine** supporting arbitrary board geometries (incl. 3D), stackable/multi‑char/emoji tiles, pluggable rules and dictionaries, fast solvers/AI, and complete bindings (Python, WASM/JS, Unity/C#, Godot/GDExtension).
> **Docs UX:** Docusaurus site with **live, in‑browser demos** using WASM + cross‑language examples.

---

## Table of Contents

* [Conventions](#conventions)
* [Monorepo Layout](#monorepo-layout)
* [Phase 0 — Project Scaffolding & CI/CD](#phase-0--project-scaffolding--cicd)
* [Phase 1 — Core Domain Model (2D MVP)](#phase-1--core-domain-model-2d-mvp)
* [Phase 2 — Rules & Scoring (Classic Crossword MVP)](#phase-2--rules--scoring-classic-crossword-mvp)
* [Phase 3 — Dictionary Engine (Unicode-first)](#phase-3--dictionary-engine-unicode-first)
* [Phase 4 — WASM Packaging & Docusaurus Playground](#phase-4--wasm-packaging--docusaurus-playground)
* [Phase 5 — FFI/C ABI & Python Bindings](#phase-5--ffic-abi--python-bindings)
* [Phase 6 — Unity & Godot Bindings](#phase-6--unity--godot-bindings)
* [Phase 7 — Non-Rectangular Boards & Custom Adjacency](#phase-7--non-rectangular-boards--custom-adjacency)
* [Phase 8 — 3D Boards](#phase-8--3d-boards)
* [Phase 9 — Stacking & Multi-Character/Emoji Tiles](#phase-9--stacking--multi-characteremoji-tiles)
* [Phase 10 — Arbitrary Move Types & Rule Plugins](#phase-10--arbitrary-move-types--rule-plugins)
* [Phase 11 — Advanced Lexica (DAWG/GADDAG), RTL, Normalization](#phase-11--advanced-lexica-dawggaddag-rtl-normalization)
* [Phase 12 — Move Generation & Solvers](#phase-12--move-generation--solvers)
* [Phase 13 — AI Opponents (Eval, Search, Difficulty)](#phase-13--ai-opponents-eval-search-difficulty)
* [Phase 14 — Persistence, Replays, Determinism](#phase-14--persistence-replays-determinism)
* [Phase 15 — Performance, Benchmarks, Fuzz](#phase-15--performance-benchmarks-fuzz)
* [Phase 16 — Packaging & Distribution](#phase-16--packaging--distribution)
* [Phase 17 — Demos (Showcase Suite)](#phase-17--demos-showcase-suite)
* [Phase 18 — Documentation Completion & Tutorials](#phase-18--documentation-completion--tutorials)
* [Appendix A — Sample Configs](#appendix-a--sample-configs)
* [Appendix B — FFI Guidelines](#appendix-b--ffi-guidelines)
* [Appendix C — Testing Strategy Overview](#appendix-c--testing-strategy-overview)

---

## Conventions

* **Rust edition:** 2024 (or latest stable at start).
* **MSRV:** set explicitly in `Cargo.toml`.
* **Linting:** `clippy -D warnings`, `rustfmt`.
* **Errors:** `thiserror` for typed errors.
* **Logging:** `tracing` + `env_logger`/`tracing-subscriber`.
* **Serialization:** `serde` (+ `serde_json` / `serde_cbor`).
* **Randomness:** `rand_chacha` with seed for determinism in tests.
* **Feature flags:** `wasm`, `ffi`, `python`, `unity`, `godot`, `simd`, `parallel`, `bench`.

---

## Monorepo Layout

```
/engine/                # Rust core crate (no platform deps)
/engine_ffi/            # C ABI wrapper over /engine
/bindings/python/       # PyO3/maturin package (wraps /engine or /engine_ffi)
/bindings/unity/        # C# thin wrapper + plugin build scripts
/bindings/godot/        # godot-rust GDExtension crate
/wasm/                  # wasm-bindgen + JS glue, npm packaging
/docs/                  # Docusaurus site with live WASM playgrounds
/examples/              # Minimal CLIs & scripts per binding
/bench/                 # Criterion benches
/test_data/             # Tiny dictionaries, board presets, fixtures
/.github/workflows/     # CI pipelines
```

---

## Phase 0 — Project Scaffolding & CI/CD

**Goal:** Establish workspace, coding standards, CI, and project skeleton.

### Implementation

* [x] Create Cargo workspace with crates above; shared `rust-toolchain.toml`.
* [x] Add `justfile` or `Makefile` with common tasks (`just build`, `just test`, `just fmt`, …).
* [x] Setup `clippy`, `rustfmt`, `cargo-deny` (licenses), `cargo-udeps`.
* [ ] GitHub Actions:

  * [x] Matrix builds (ubuntu-latest, macos-latest, windows-latest).
  * [x] `cargo test --all-features` + `clippy` + `fmt --check`.
  * [x] Cache cargo & target.
* [x] Pre-commit hooks: fmt & clippy on staged Rust files.
* [x] License & CODEOWNERS; CONTRIBUTING.md; SECURITY.md.
* [x] Issue templates & PR templates.

### Unit Tests

* N/A (scaffolding). Add a placeholder `#[test]` in `/engine`.

### Docs

* [x] `/docs` Docusaurus bootstrap (classic template).
* [x] “Architecture Overview” stub page.
* [x] “Contributing” page.

### Demo

* None.

### Exit Criteria

* CI green on 3 OSes; repo bootstrapped; style gates enforced.

---

## Phase 1 — Core Domain Model (2D MVP)

**Goal:** Minimal playable 2D rectangular board model & state types (no scoring/lexicon yet).

### Implementation

* **Core types** (`engine`):

  * [x] `Symbol`: Unicode scalar or grapheme (`String`); canonicalized via Unicode NFC.
  * [x] `TileKind { id: String, symbol: String, score: i16, is_blank: bool, aliases: Vec<String> }`.
  * [x] `Tile { kind_id: String, mark: Option<String> /* e.g., user marks */ }`.
  * [x] `CellId` newtype; `Coord2D { x:i32, y:i32 }`.
  * [x] `Cell { stack: Vec<Tile> }` (stacking empty for now but struct prepared).
  * [x] `BoardGeometry` trait:
    \- neighbors(CellId) -> SmallVec<CellId>
    \- to\_cell\_id(Coord2D) / from\_cell\_id(CellId) -> Option<Coord2D>
  * [x] `RectGridGeometry { width, height }` implements `BoardGeometry` (orthogonal neighbors).
  * [x] `Bonus { letter_mul: i8, word_mul: i8, tags: BTreeSet<String> }`.
  * [x] `Board { geom, cells: Vec<Cell>, bonuses: HashMap<CellId, Bonus> }`.
  * [x] `Rack { tiles: Vec<Tile> }`, `Bag { counts: HashMap<TileKind, u32>, rng }`.
  * [x] `PlayerId`, `Player { rack, score }`.
  * [x] `GameConfig { tileset, rack_size, board_layout, ruleset_id, dictionary_id }`.
  * [x] `GameState { board, players, to_move: PlayerId, bag, turn_num }`.
  * [x] `MoveDraft` (unvalidated): `placements: Vec<(CellId, Tile)>` + metadata.
  * [x] Error types: `EngineError` variants (InvalidCell, Collision, …).
* **APIs**

  * [x] `Game::new(config) -> GameState`.
  * [x] `bag.draw(n) -> Vec<Tile>`.
  * [x] Place without validation: `state.preview(draft) -> Preview { new_board }` (internal).
  * [x] Deterministic RNG seed in config for reproducible tests.

### Unit Tests

* [x] Geometry: neighbor sets on edges/corners for several board sizes.
* [x] Board indexing: `to_cell_id/from_cell_id` roundtrips.
* [x] Bag: distribution sums; drawing depletes; determinism with seed.
* [x] Rack: add/remove tiles; capacity enforcement (rack\_size).
* [x] Bonus map: default 1×; attach tags; retrieval.

### Docs

* [x] “Core Model” page: types & diagrams (stub).
* [x] Config example in docs (Rust snippet).
* [x] Example: create game, draw tiles, print board (Rust snippet).

### Demo

* [x] CLI example: create 7×7 board, draw rack, render ASCII grid (no words yet) — see cargo example `ascii_demo`.
* [x] Docs page snippet shows usage.

### Exit Criteria

* Types stable; deterministic bag; 2D rect geometry works; tests pass.

---

## Phase 2 — Rules & Scoring (Classic Crossword MVP)

**Goal:** Validate placements, enforce connectivity, compute score with bonuses (no dictionary yet → optionally “free word mode” flag).

### Implementation

* [x] `Rules` trait:

  * `validate(state: &GameState, draft: &MoveDraft) -> Result<ValidatedMove>`
  * `score(state: &GameState, mv: &ValidatedMove) -> ScoreBreakdown`
  * `commit(state: &mut GameState, mv: ValidatedMove, score: ScoreBreakdown)`
* [x] `CrosswordRules` (2D):

  * Validate line straightness (row or column), contiguity, anchor (must touch existing tiles unless first move center).
  * Generate **all formed sequences** (main + cross words) from placement.
  * Apply bonuses: letter bonuses apply to newly placed tiles only; word bonuses multiply whole word.
  * “Free word mode” toggle to skip dictionary check (until Phase 3).
  * Bingo bonus (configurable).
* [x] Remove used tiles from rack; refill from bag.

### Unit Tests

* [x] Validate straight-line & contiguity errors.
* [x] First move must cover center (config).
* [x] Anchor requirement on subsequent moves.
* [x] Score examples covering: letter/word multipliers; multiple words.
* [x] Rack updates & refill; bag underflow behavior (basic).

### Docs

* [x] “Scoring & Validation” stub page.
* [x] Config keys note (center rule, bingo) inline with demo.

### Demo

* [x] CLI: play a fixed sequence of moves; print score breakdowns (see `scoring_demo`).
* [ ] Add a static docs page with **animated** move breakdown (SVG frames or GIF).

### Exit Criteria

* End-to-end classic placement (without lexicon) works; scoring verified by tests.

---

## Phase 3 — Dictionary Engine (Unicode-first)

**Goal:** Word validation via pluggable lexica.

### Implementation

* [x] `Dictionary` trait:

  * `contains(word: &[Symbol]) -> bool`
  * `has_prefix(prefix: &[Symbol]) -> bool` (for solvers later; may stub).
* [x] `SetDictionary` (HashSet) for small test lexica with NFC + optional case fold.
* [x] Loader from files/paths and basic metadata (case fold, min/max length). Language/advanced metadata later.
* [x] Integrate with `CrosswordRules`: dictionary check on formed words (free_word_mode=false).

### Unit Tests

* [x] Word membership positive/negative.
* [x] Normalization: composed vs decomposed forms equal.
* [x] Case folding toggles.
* [x] Integration: invalid word move rejected; valid accepted.

### Docs

* [x] “Lexicon Integration” stub page: normalization + case folding notes.
* [ ] Guidance on licensing/packaging word lists (ship tiny test lists, document how to plug real ones).

### Demo

* [x] CLI: try to play illegal word → engine rejects with clear error (`illegal_word_cli` example).
* [x] Docs: interactive check superseded by the WASM Playground; dictionary usage documented.

### Exit Criteria

* Dictionary-backed validation on; tests cover Unicode & normalization.

---

## Phase 4 — WASM Packaging & Docusaurus Playground

**Goal:** Build a **WASM module** from `/engine`, surface JS API, and embed **live playground** into docs.

### Implementation

* [x] `wasm/` crate with `wasm-bindgen` wrappers:

  * Expose: `new_game(configJson, players)`, `play_move(json) -> result`, `get_board()`.
  * JSON-serialized interop; artifacts copied to docs/static.
* [x] Build script: `make wasm` (`wasm-pack build --target web`); copies artifacts into `/docs/static/wasm/engine/`.
* [ ] Docusaurus:

  * Custom **React Playground component** that loads WASM, renders a board (Canvas/SVG), supports drag‑drop placement.
  * Add code tabs (Rust / JS usage).
* [ ] Web worker optional: move compute to worker if needed.

### Unit Tests

* [x] `wasm-bindgen-test` smoke test for exported API.
* [ ] Golden tests: `getBoard()` JSON snapshots for known states.

### Docs

* [x] “Web (WASM) API” page with example usage.
* [x] Live Playground page (React) that loads WASM and places a word.

### Demo

* [x] Docs homepage hero: **Try it now** button opens Playground.
* [x] “Place a word” walkthrough via Playground page.

### Exit Criteria

* Docs site builds; in-browser engine demo works without errors.

---

## Phase 5 — FFI/C ABI & Python Bindings

**Goal:** Stable **C ABI** and **Python** package with full parity for existing features.

### Implementation

* **engine\_ffi/**

  * [ ] Opaque handles (`EngineHandle`, `GameHandle`, `ErrorHandle`).
  * [ ] `extern "C"` no‑mangle fns: create/free engine, new game from JSON config, play move from JSON, query board as JSON.
  * [ ] `cbindgen` to generate `engine.h`.
  * [ ] Error handling: last‑error string per-thread or explicit `ErrorHandle`.
* **bindings/python/**

  * [ ] PyO3/maturin project: `pip install .` yields `wordengine` module.
  * [ ] Pythonic API mirrors WASM API (plus conveniences).
  * [ ] Wheels for macOS, Linux (manylinux), Windows (CI later).

### Unit Tests

* [ ] C smoke test: link against shared lib, call `new_game`, `play_move`.
* [ ] Python `pytest`: create/commit moves; error paths.
* [ ] Conformance suite: load shared JSON fixtures and assert identical results (Rust vs Python vs WASM).

### Docs

* [ ] “C ABI” page with `engine.h` excerpt & lifecycle diagram.
* [ ] “Python” quickstart with examples (CLI mini-game).

### Demo

* [ ] `examples/python/mini_cli.py`: human vs human on terminal.
* [ ] Docs page with runnable Python snippets (via embedded code + output screenshots).

### Exit Criteria

* ABI stable; Python parity; conformance tests pass across Rust/WASM/Python.

---

## Phase 6 — Unity & Godot Bindings

**Goal:** Unity (C#) and Godot (GDExtension) integrations with basic UIs.

### Implementation

* **Unity**

  * [ ] Build shared libs for platforms (dev: Win/Mac/Linux).
  * [ ] C# wrapper (DllImport) for select API.
  * [ ] Minimal 2D board UI (UGUI) calling native plugin.
* **Godot**

  * [ ] godot-rust `gdext` crate exposing `WordEngine` class with `new_game`, `play_move`.
  * [ ] Godot scene for 2D board, inspector-exposed config.

### Unit Tests

* [ ] Unity playmode tests (basic engine calls).
* [ ] Godot script tests (GUT or simple asserts) calling `WordEngine`.

### Docs

* [ ] “Unity Integration” page: folder layout, import steps, code sample.
* [ ] “Godot Integration” page: building, registering classes, GDScript sample.

### Demo

* [ ] Unity sample scene (drag tiles, place word).
* [ ] Godot sample project (same scenario).

### Exit Criteria

* Unity & Godot demos run locally, perform a complete move cycle.

---

## Phase 7 — Non-Rectangular Boards & Custom Adjacency

**Goal:** Support **arbitrary board graphs** (hex, triangular, holes).

### Implementation

* [ ] `BoardGeometry` extended:

  * Accepts **graph-based geometry**: `BoardGraph { nodes: Vec<Node>, edges: Vec<(u32,u32,DirTag)> }`.
  * `Direction` tags for UI hints (N/E/S/W/NE/…).
* [ ] Loader: geometry from JSON/TOML (coordinates optional; UI layer may use them).
* [ ] Validation adapts: contiguity & “line” detection generalized via path checks on the graph (define “line” as path with consistent `DirTag` or allow non-linear per ruleset).

### Unit Tests

* [ ] Hex grid: neighbor counts; path collinearity along a direction family.
* [ ] Board with holes: ensure spelling lines skip blocked cells correctly.
* [ ] Performance: large sparse graph validate within bounds.

### Docs

* [ ] “Custom Geometry” with diagrams and sample configs for hex & triangle boards.
* [ ] Guidance on mapping graph coords to 2D positions for rendering.

### Demo

* [ ] Docs Playground: toggle between Rect & Hex geometry; place sample words.
* [ ] Godot/Unity scenes for Hex board.

### Exit Criteria

* Engine validates words on non-rect grids; demos reflect adjacency changes.

---

## Phase 8 — 3D Boards

**Goal:** Add **z-dimension** boards (layered grids; true 3D paths).

### Implementation

* [ ] `Coord3D { x,y,z }`; `Grid3DGeometry` neighbors (6 axis or 26 with diagonals—configurable).
* [ ] Bonuses support per cell in 3D.
* [ ] Rules: “line” can be along x, y, or z (configurable diagonals).
* [ ] UI adapters: serialization of 3D board slices for frontends.

### Unit Tests

* [ ] Validate 3D straightness; multiple layered word formation.
* [ ] Bonuses in 3D applied correctly.
* [ ] Boundary cases: edges/corners across layers.

### Docs

* [ ] “3D Boards” page with rendered slices and a 3D explanation.
* [ ] Config examples for 3×3×3 toy board.

### Demo

* [ ] Godot 3D scene (basic) to visualize stacked layers.
* [ ] Docs: 2D slice selector (z slider) in Playground.

### Exit Criteria

* 3D validated placements & scoring proven with tests & demo.

---

## Phase 9 — Stacking & Multi-Character/Emoji Tiles

**Goal:** Implement **stackable cells** (Upwords-like) & **multi-char/emoji** tiles.

### Implementation

* [ ] `Cell.stack: Vec<Tile>` activated; top letter determines symbol by default (pluggable).
* [ ] Stacking rules:

  * Max height; allowed overlays (e.g., cannot place same symbol).
  * Scoring variants: sum all stack tiles vs top-only; config toggles.
* [ ] Multi-character tiles:

  * Treat `TileKind.symbol` as grapheme sequence.
  * Word reading uses **grapheme segmentation** (`unicode-segmentation` crate).
* [ ] Blanks with **explicit mapping**: a blank tile carries runtime `as_symbol`.

### Unit Tests

* [ ] Upwords scenario reproductions: overlay to create new words; forbidden overlays.
* [ ] Mixed emoji + letters; grapheme cluster tests (skin tones, ZWJs).
* [ ] Blanks: mapping persists; scoring unaffected by tile base.

### Docs

* [ ] “Stacking Rules” with visuals; “Emoji & Graphemes” caveats (ZWJ, NFC).
* [ ] Config snippets enabling Upwords-like mode.

### Demo

* [ ] Docs Playground toggle: “Stacking ON”.
* [ ] Python example: create emoji crossword; print board with unicode.

### Exit Criteria

* Stacking and multi-char tiles fully supported & tested.

---

## Phase 10 — Arbitrary Move Types & Rule Plugins

**Goal:** Framework for **custom actions** beyond simple placement.

### Implementation

* [ ] `Action` enum: `Place`, `Stack`, `SwapRack`, `RotateTile`, `SlideGroup`, `Custom(String, Value)`.
* [ ] `RulePlugin` trait: hooks (`pre_validate`, `validate`, `score_hooks`, `commit_hooks`).
* [ ] Move pipeline: convert `UserMove { actions: Vec<Action> }` → `ValidatedMove` via plugin chain.
* [ ] Built-in plugins: classic crossword, Upwords, freeform anagram mode, diagonal-allowed mode.

### Unit Tests

* [ ] Compose multiple plugins; ensure deterministic order & effects.
* [ ] Invalid action sequences rejected with precise errors.
* [ ] Score hooks accumulate modifiers predictably.

### Docs

* [ ] “Rule Plugins” developer guide with lifecycle diagram.
* [ ] Template plugin crate example.

### Demo

* [ ] Docs Playground: “Enable Diagonals” and “Anagram Mode” toggles.
* [ ] Unity/Godot: button to switch rules mid-session (if allowed by config).

### Exit Criteria

* Custom rule pipeline proven; at least 3 plugins included & documented.

---

## Phase 11 — Advanced Lexica (DAWG/GADDAG), RTL, Normalization

**Goal:** Efficient lexicon structures & multilingual robustness.

### Implementation

* [ ] `DawgDictionary` with prefix search.
* [ ] `Gaddag` (for fast Scrabble move gen): word storage & iterator API.
* [ ] RTL support:

  * Config per language: reading direction; UI rendering hints.
  * Validation respects directionality; still canonicalize codepoints.
* [ ] Normalization modes: NFC default; optional NFKC; locale-sensitive case rules.
* [ ] Tokenization hooks: per-language grapheme/cluster segmentation.

### Unit Tests

* [ ] DAWG correctness vs SetDictionary; prefix queries.
* [ ] GADDAG path enumeration; known word coverage.
* [ ] RTL word assembly tests (Hebrew/Arabic samples).
* [ ] Normalization permutations accepted equivalently.

### Docs

* [ ] “Dictionary Engines: DAWG & GADDAG”.
* [ ] “Language Packs: RTL & Normalization”.

### Demo

* [ ] Docs: benchmark chart (DAWG vs SetDictionary lookups) using precomputed data.
* [ ] Live demo: switch dictionary engine in Playground.

### Exit Criteria

* DAWG/GADDAG pass parity tests; RTL validated; docs updated.

---

## Phase 12 — Move Generation & Solvers

**Goal:** Full **anchor + cross-check** move generation for 2D/3D; supports stacking & multi-chars; blank handling.

### Implementation

* [ ] Cross-check computation per open cell (legal symbols by perpendicular checks).
* [ ] Anchor discovery on current board.
* [ ] Enumerator:

  * Left/Right (or negative/positive direction) expansion using DAWG/GADDAG.
  * Rack multiset with blanks; multi-char tile emission.
  * Pruning via cross-checks & board bounds; scoring inline.
* [ ] API: `generate_moves(state, rack) -> Vec<CandidateMove>` with score & metadata.

### Unit Tests

* [ ] Reproduce known Scrabble positions (golden expected move lists).
* [ ] Stress tests with long racks, many anchors.
* [ ] 3D move gen basic correctness.
* [ ] Stacking-aware cross-checks.

### Docs

* [ ] “Move Generation” deep dive with animations.
* [ ] Complexity notes & tuning parameters.

### Demo

* [ ] Docs Playground: “Show legal moves” overlay; click to play.
* [ ] Python script: top 10 moves for sample position.

### Exit Criteria

* Generator returns correct sets; performance acceptable for standard boards.

---

## Phase 13 — AI Opponents (Eval, Search, Difficulty)

**Goal:** Solid AI baseline + difficulty knobs.

### Implementation

* [ ] Evaluation: raw score + **rack leave** (static table) + board equity (simple heuristics).
* [ ] Tie-breakers: leave balance (vowels/consonants), openings/defense heuristic.
* [ ] Search:

  * Greedy baseline.
  * Optional one-ply lookahead (simulate opponent’s best reply from reduced set).
  * Time/Node budget & randomness for difficulty levels.
* [ ] Hint API: `best_move(state, rack, level) -> CandidateMove`.

### Unit Tests

* [ ] Determinism with seed; identical results across bindings.
* [ ] Edge: endgame (exhaust bag) penalty handling.
* [ ] Hints never illegal; bounded runtime in tests.

### Docs

* [ ] “AI Overview” & difficulty profiles.
* [ ] “Using AI in bindings” code samples.

### Demo

* [ ] Docs: play vs CPU (easy/medium).
* [ ] Unity/Godot: CPU opponent toggle.

### Exit Criteria

* AI plays competently; difficulty settings distinguishable; tests deterministic.

---

## Phase 14 — Persistence, Replays, Determinism

**Goal:** Save/Load, reproducible games, and replay logs.

### Implementation

* [ ] `serde` snapshots of `GameState` + compact **CBOR** representation.
* [ ] **Zobrist hashing** of positions for replay integrity.
* [ ] Event log (append-only): draw, play, exchange, pass.
* [ ] Deterministic RNG seeded in `GameConfig`.

### Unit Tests

* [ ] Round-trip save/load equality.
* [ ] Replays reproduce identical final state.
* [ ] Hash changes on any legal modification.

### Docs

* [ ] “Persistence & Replays” guide.
* [ ] Versioning & forward-compat strategy.

### Demo

* [ ] Docs: download/save game JSON; reload into Playground.

### Exit Criteria

* Stable snapshots; replays deterministic across platforms.

---

## Phase 15 — Performance, Benchmarks, Fuzz

**Goal:** Measure & harden.

### Implementation

* [ ] `criterion` benches: dictionary lookups, cross-checks, move gen, AI.
* [ ] `rayon` optional parallelism on evaluation.
* [ ] `cargo-fuzz`: fuzz parsers (config), move validation pipeline.
* [ ] Memory profiling notes; optional `simd` gates.

### Unit/Bench

* [ ] Bench thresholds recorded (CI prints previous vs current).
* [ ] Fuzz targets run on CI (smoke, not soak).

### Docs

* [ ] “Performance” page with charts (generated offline & embedded).
* [ ] “Tuning knobs” (parallelism, pruning).

### Demo

* [ ] Docs: run a pre-canned benchmark button → shows results table (static JSON).

### Exit Criteria

* No panics under fuzz seeds; perf within targets; docs explain trade-offs.

---

## Phase 16 — Packaging & Distribution

**Goal:** Publishable artifacts for all platforms.

### Implementation

* [ ] Rust crate publish (crates.io).
* [ ] Python wheels (manylinux, macOS, Windows) via maturin; upload to PyPI.
* [ ] WASM npm package with TS types.
* [ ] Unity package: `.unitypackage` or UPM; platform libs prebuilt.
* [ ] Godot: prebuilt GDExtension for common OS; template project.
* [ ] Versioning & CHANGELOG; SemVer commitment.

### Tests

* [ ] Install from artifacts in clean env; smoke sample runs.

### Docs

* [ ] “Install” pages per platform.
* [ ] Version matrix & support policy.

### Demo

* [ ] Copy/paste one-liners to run demos after install.

### Exit Criteria

* Consumers can `pip install`, `npm i`, import Unity/Godot packages and run samples.

---

## Phase 17 — Demos (Showcase Suite)

**Goal:** Polished demos proving breadth.

### Implementation & Demos

* [ ] **Classic Crossword** (web + Unity + Godot): AI opponent.
* [ ] **Hex Board Variant** (web): show non-rect adjacency.
* [ ] **3D Board** (Godot): slice UI; valid 3D words.
* [ ] **Upwords-like Stacking** (web): toggle stacking; sample puzzles.
* [ ] **Emoji Crossword** (web + Python CLI): shows graphemes/ZWJs.
* [ ] **Solver Tool** (web + Python): input rack → best moves.

### Tests

* [ ] Demo e2e tests (Playwright) for web Playground: place word, undo, redo.

### Docs

* [ ] “Showcase” pages with embedded demos & source links.

### Exit Criteria

* All showcase demos load fast, are stable, and highlight key features.

---

## Phase 18 — Documentation Completion & Tutorials

**Goal:** High-quality docs with hands-on tutorials.

### Implementation

* [ ] Docusaurus structure:

  * **Concepts:** Boards, Tiles, Rules, Lexica, Move Gen, AI.
  * **How-tos:** Create variant; add emoji language; enable 3D; write plugin.
  * **Reference:** Rust API (rustdoc), C ABI, Python, WASM, Unity, Godot.
  * **Playgrounds:** Each major concept has a small interactive.
* [ ] Add “Cookbook” snippets per binding.

### Tests

* [ ] Link checker; code snippet tests (doctests for Rust; CI runs Python snippets).

### Exit Criteria

* Docs provide beginner → advanced path; all snippets compile/run.

---

## Appendix A — Sample Configs

### Minimal Rect Board (JSON)

```json
{
  "tileset": {
    "tile_kinds": [
      {"id":"A","symbol":"A","score":1,"count":9},
      {"id":"BLANK","symbol":" ","score":0,"is_blank":true,"count":2}
    ]
  },
  "rack_size": 7,
  "board_layout": {
    "type": "rect",
    "width": 15,
    "height": 15,
    "bonuses": {"(7,7)":{"word_mul":2}}
  },
  "ruleset_id": "crossword_classic",
  "dictionary_id": "en_test_small",
  "rng_seed": 42
}
```

### Hex Geometry (TOML)

```toml
[board_layout]
type = "graph"
nodes = [{id=0,x=0,y=0}, {id=1,x=1,y=0}, {id=2,x=0,y=1}]
edges = [{a=0,b=1,dir="E"}, {a=0,b=2,dir="SE"}, {a=1,b=2,dir="SW"}]
```

---

## Appendix B — FFI Guidelines

* **Opaque handles** only; no raw Rust pointers exposed.
* **Ownership:** Every `create_*` has matching `free_*`. Document lifetimes.
* **Error strategy:** Functions return int code; separate `last_error()` per thread or explicit output buffer.
* **ABI stability:** Use `repr(C)` on structs shared via FFI; prefer JSON strings for complex data.
* **Threading:** Document thread-safety; WASM build may be single-threaded.
* **Headers:** Generate with `cbindgen`; never hand-edit.
* **Versioning:** Include `engine_version()` and `engine_abi_version()`.

---

## Appendix C — Testing Strategy Overview

* **Unit tests:** Per module; pure logic with small fixtures.
* **Golden tests:** JSON snapshots for board states & move lists.
* **Property tests:** proptest for bag draws, normalization, graph connectivity.
* **Fuzz:** Move parser/validator, config loader.
* **Conformance tests:** Same JSON scenarios executed by Rust, WASM, Python; results compared byte-for-byte.
* **E2E (web):** Playwright tests of Playground (place, invalid, score).
* **Performance gates:** Criterion baselines logged; detect 2× regressions.
* **Determinism:** Seeded RNG; tests assert identical outcomes across runs.

---

## Documentation Pages Checklist (running list)

* [ ] Getting Started (Rust / Python / Web / Unity / Godot).
* [ ] Concepts: Tiles, Boards (2D/3D/graph), Rules, Dictionaries.
* [ ] Unicode & Emoji: graphemes, normalization, RTL.
* [ ] Move Validation & Scoring (animated examples).
* [ ] Dictionary Engines: Set, DAWG, GADDAG.
* [ ] Move Generation: anchor, cross-checks, pruning.
* [ ] AI: evaluation, difficulty, time controls.
* [ ] Persistence & Replays.
* [ ] Performance & Benchmarks.
* [ ] Extending with Rule Plugins.
* [ ] API Reference (autogenerated + human).
* [ ] Live Playgrounds (per concept).
* [ ] Showcase Demos.

---

### Notes & Risks

* **Lexicon licensing:** ship only tiny test lists; document how to import real wordlists; allow user-provided files.
* **Unicode complexity:** default NFC; expose toggles; add grapheme tests for ZWJ/skin-tone modifiers.
* **WASM bundle size:** keep dictionary out of bundle by default; allow user upload; provide small demo lexicon.
* **Cross-platform builds:** automate with GitHub Actions; cache toolchains; test artifact installability.
* **Performance:** DAWG/GADDAG critical; consider memory-reducing encodings (succinct tries) later if needed.

---

This plan is intentionally **modular and milestone-driven**: each phase yields a **working artifact**, with **tests, docs, and a demo** to validate progress and keep bindings in sync.
