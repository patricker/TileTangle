---
id: rule-plugins
title: Rule Plugins
description: Compose custom move behaviors via a simple plugin pipeline.
---

The engine supports an opt-in plugin pipeline that wraps base crossword rules. Plugins can:

- Reject or transform user actions (pre-validate/validate)
- Convert actions into a `MoveDraft` (placements) consumed by base rules
- Modify the computed score (`modify_score` hook)
- Run post-commit side effects

Core types

- `Action`: `Place`, `Stack`, `SwapRack`, `RotateTile`, `SlideGroup`, `Custom(String, Value)`
- `UserMove { actions: Vec<Action> }`
- `RulePlugin` trait with hooks: `pre_validate`, `validate`, `to_draft`, `modify_score`, `commit`
- `PluginRules { base: CrosswordRules, plugins: Vec<Box<dyn RulePlugin>> }`

Example: Basic actions + score bonus

```rust
use engine::*;

let tileset = Tileset { tile_kinds: vec![
    TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
    TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
]};
let mut counts = std::collections::HashMap::new(); counts.insert("A".to_string(), 10); counts.insert("B".to_string(), 10);
let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 1, tile_counts: counts };
let mut state = GameState::new(&cfg, 2).unwrap();
state.dictionary = Some(Box::new(FstDictionary::from_words(vec!["AB".to_string()], true)));

let base = CrosswordRules { free_word_mode: false, ..Default::default() };
let rules = PluginRules::new(base, vec![
  Box::new(BasicActionsPlugin::default()),
  Box::new(ScoreBonusPlugin { bonus: 5 }),
]);

let center = CrosswordRules::center_cell(&state.board.geom);
let cc = state.board.geom.from_cell_id(center).unwrap();
let mut mv = UserMove { actions: vec![
  Action::Place { x: cc.x, y: cc.y, kind_id: "A".into(), mark: None },
  Action::Place { x: cc.x + 1, y: cc.y, kind_id: "B".into(), mark: None },
]};

let v = rules.validate_user_move(&state, mv).unwrap();
let mut sc = rules.score_user_move(&state, &v);
assert_eq!(sc.total, 9); // 1+3 + bonus 5
rules.commit_user_move(&mut state, v, &sc).unwrap();
```

Lifecycle

```
UserMove (actions)
   |
   | pre_validate()  — plugins may reject or rewrite actions
   v
validate()          — plugins may enforce constraints
   |
   | to_draft()     — one plugin produces MoveDraft (placements)
   v
CrosswordRules.validate()  — base line/contiguity/anchor checks
   |
   v
score()             — base scoring
   |
   | modify_score() — plugins adjust totals/breakdown
   v
commit()            — base commit (board/rack/bag updates)
   |
   v
plugin.commit()     — optional side effects
```

Template plugin crate

- See `examples/plugin_template/` for a minimal external crate that implements two plugins:
  - `ScoreBonusPlugin` — adds a fixed bonus to each move.
  - `RejectFarPlacementsPlugin` — rejects moves when placed tiles are too far apart.
  - Import the crate and add `Box::new(...)` instances to your `PluginRules` list.

Notes

- Plugins run in order. Be mindful of deterministic behavior.
- Use `Custom(String, Value)` to pass structured configs to specialized plugins.
- Upwords-style overlays are supported in base rules (see Stacking). A plugin can toggle these at runtime or pre-process actions.
