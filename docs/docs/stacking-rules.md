---
sidebar_position: 13
---

# Stacking Rules

TileTangle supports Upwords-like stacking:

- Enable stacking and set constraints on the rules:
  - `stacking_enabled`: allow placing on occupied cells.
  - `stacking_max_height`: maximum stack height per cell.
  - `forbid_same_symbol_overlay`: disallow placing the same symbol on top.
  - `stacking_scoring`: `TopOnly` (default) or `SumStack`.

## Web (WASM)

```ts
const g = mod.new_game(JSON.stringify(cfg), 2);
mod.set_stacking(g, /*enabled*/ true, /*max*/ 7, /*forbidSame*/ true, /*sumStack*/ true);
```

## Rust

```rust
let mut rules = engine::CrosswordRules { stacking_enabled: true, stacking_max_height: 5, forbid_same_symbol_overlay: true, stacking_scoring: engine::StackScoring::SumStack, ..Default::default() };
```

## Notes

- Bonuses: letter bonuses apply to newly placed tiles; word multipliers multiply the whole word.
- When `SumStack` is used, underlying stack tiles contribute their base score (unmultiplied).

