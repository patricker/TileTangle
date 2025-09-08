---
sidebar_position: 4
---

# Core Model (2D MVP)

Key types (engine crate):

- `RectGridGeometry` with `neighbors`, `to_cell_id`, `from_cell_id`.
- `Board { geom, cells, bonuses }` and `Cell { stack }`.
- `Symbol` (NFC), `TileKind`, `Tile`, `Rack`, `Bag` (seeded RNG).
- `GameConfig`, `GameState`, `MoveDraft`.

Quickstart (Rust):

```rust
use engine::*;
let tileset = Tileset { tile_kinds: vec![
    TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
]};
let cfg = GameConfig {
  tileset,
  rack_size: 7,
  board_layout: RectBoardLayout { width: 7, height: 7 },
  ruleset_id: "crossword_classic".into(),
  dictionary_id: "en_test".into(),
  rng_seed: 1,
  tile_counts: std::collections::HashMap::from([("A".into(), 10u32)]),
};
let state = GameState::new(&cfg, 2)?;
let c = state.board.geom.to_cell_id(Coord2D { x: 3, y: 3 }).unwrap();
let draft = MoveDraft { placements: vec![(c, Tile { kind_id: "A".into(), mark: None })] };
let preview = state.preview(&draft)?;
```

Run the ASCII demo:

```
cargo run --example ascii_demo -p tiletangle-engine
```
