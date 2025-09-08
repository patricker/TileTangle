#!/usr/bin/env python3
import json
try:
    from tiletangle import Game
except ImportError:
    print("ERR: run `maturin develop` in bindings/python first.")
    raise

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
print("Board:", g.get_board_json())
mv = json.dumps([
    {"x": 2, "y": 2, "kind_id": "A"},
    {"x": 3, "y": 2, "kind_id": "B"},
])
print("Score:", g.play_move(mv))

