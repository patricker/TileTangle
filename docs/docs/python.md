---
title: Python
---

TileTangle ships Python bindings via PyO3.

Install for development:

- `pip install maturin`
- `cd bindings/python && maturin develop --release`

Quickstart:

```
from tiletangle import Game
import json

cfg = {
  "tileset": {"tile_kinds": [
    {"id": "A", "symbol": "A", "score": 1},
    {"id": "B", "symbol": "B", "score": 3},
  ]},
  "rack_size": 7,
  "board_layout": {"width": 5, "height": 5},
  "ruleset_id": "cross",
  "dictionary_id": "en",
  "rng_seed": 42,
  "tile_counts": {"A": 10, "B": 10},
  "free_word_mode": True,
}
g = Game(json.dumps(cfg), 2)
print(g.get_board_json())
print(g.play_move(json.dumps([
  {"x": 2, "y": 2, "kind_id": "A"},
  {"x": 3, "y": 2, "kind_id": "B"},
])))
```

Hints and difficulty:

```python
# Quick difficulty preset
best = g.best_move("medium", seed=12345)
if best:
    print(best)

# Tuned greedy with budgets
hint = g.best_move_greedy(max_len=7, lookahead_depth=1, node_limit=64, time_limit_ms=25)

# Opponent visibility for look-ahead (omniscient vs. hidden via bag sampling)
best_hidden = g.best_move("hard", seed=1, opponent="bag")
hint_hidden = g.best_move_greedy(lookahead_depth=1, opponent="bag")

# Heuristic breakdown for a manual placement
eval = g.evaluate_candidate('[{"x":2,"y":2,"kind_id":"A"}]', difficulty="medium")
print(eval)
```
