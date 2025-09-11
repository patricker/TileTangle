#!/usr/bin/env python3
import json
try:
    from tiletangle import Game
except ImportError:
    print("ERR: run `maturin develop` in bindings/python first.")
    raise

cfg = {
    "tileset": {"tile_kinds": [
        {"id": "SMILE", "symbol": "😀", "score": 1},
        {"id": "GRIN", "symbol": "😁", "score": 2},
        {"id": "BLANK", "symbol": " ", "score": 0, "is_blank": True},
    ]},
    "rack_size": 7,
    "board_layout": {"width": 5, "height": 5},
    "ruleset_id": "cross",
    "dictionary_id": "en",
    "rng_seed": 7,
    "tile_counts": {"SMILE": 10, "GRIN": 10, "BLANK": 2},
    "free_word_mode": True,
}

g = Game(json.dumps(cfg), 2)
print("Board:", g.get_board_json())
mv = json.dumps([
    {"x": 2, "y": 2, "kind_id": "SMILE"},
    {"x": 3, "y": 2, "kind_id": "GRIN"},
])
print("Score:", g.play_move(mv))
print("Board:", g.get_board_json())

