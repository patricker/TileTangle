#!/usr/bin/env python3
"""Print the top candidate moves for a sample position."""
import json
import sys

try:
    from tiletangle import Game
except ImportError:  # pragma: no cover - guidance for developers
    print("ERR: run `maturin develop` in bindings/python first.")
    sys.exit(1)

cfg = {
    "tileset": {"tile_kinds": [
        {"id": "A", "symbol": "A", "score": 1},
        {"id": "E", "symbol": "E", "score": 1},
        {"id": "I", "symbol": "I", "score": 1},
        {"id": "N", "symbol": "N", "score": 1},
        {"id": "O", "symbol": "O", "score": 1},
        {"id": "R", "symbol": "R", "score": 1},
        {"id": "S", "symbol": "S", "score": 1},
        {"id": "T", "symbol": "T", "score": 1},
    ]},
    "rack_size": 7,
    "board_layout": {"width": 5, "height": 5},
    "ruleset_id": "cross",
    "dictionary_id": "en",
    "rng_seed": 7,
    "tile_counts": {k: 5 for k in "AEINORST"},
    "free_word_mode": False,
}

dict_words = [
    "AT", "ATE", "EAT", "TEA", "TONE", "STONE", "SATE", "ARISE", "RATION",
    "IRON", "SENIOR", "STOANE", "NOTES",
]

game = Game(json.dumps(cfg), 2)
# Deterministic rack for the first player
rack_letters = list("STONERA")
game.set_rack(rack_letters)
game.set_dictionary_from_words(dict_words, True)

moves = game.generate_moves(7, 10)
if not moves:
    print("No moves available with rack", rack_letters)
    sys.exit(0)

best = game.best_move("medium", seed=12345)
if best:
    print("CPU suggestion (medium):", best["word"], f"{best['total']} pts")
    print(
        "  components:",
        f"score={best['score']}",
        f"leave={best['rack_leave']}",
        f"equity={best['board_equity']}",
        f"endgame={best['endgame_penalty']}",
    )
    print("  placements:", best["placements"])

greedy = game.best_move_greedy(
    max_len=7,
    lookahead_depth=1,
    seed=12345,
    node_limit=32,
    time_limit_ms=10,
)
if greedy and greedy["word"] != (best["word"] if best else None):
    print(
        "Fallback greedy move:",
        greedy["word"],
        f"{greedy['total']} pts",
    )

log = game.event_log()
print(f"Event log entries: {len(log)}")

print("Top candidate moves (word, score, placements):")
for idx, mv in enumerate(moves, start=1):
    placements = [f"({p['x']}, {p['y']}, {p['kind_id']})" for p in mv["placements"]]
    print(f"{idx:>2}. {mv['word']} — {mv['score']} pts :: {' '.join(placements)}")
