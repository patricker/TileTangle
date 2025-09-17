#!/usr/bin/env python3
"""Demonstrate emoji tile support by playing a short free-word turn."""
import json
import sys

try:
    from tiletangle import Game
except ImportError:  # pragma: no cover
    print("ERR: run `maturin develop` (bindings/python) to build the wheel first.")
    sys.exit(1)

CONFIG = {
    "tileset": {"tile_kinds": [
        {"id": "SUN", "symbol": "🌞", "score": 4},
        {"id": "STAR", "symbol": "⭐", "score": 3},
        {"id": "HEART", "symbol": "❤️", "score": 2},
        {"id": "SMILE", "symbol": "😀", "score": 1},
        {"id": "MOON", "symbol": "🌙", "score": 2},
        {"id": "NOTE", "symbol": "🎵", "score": 3},
        {"id": "PLANET", "symbol": "🪐", "score": 5},
        {"id": "BL", "symbol": "✨", "score": 0, "is_blank": True},
    ]},
    "rack_size": 7,
    "board_layout": {"width": 7, "height": 7},
    "ruleset_id": "emoji",
    "dictionary_id": "emoji",
    "rng_seed": 99,
    "tile_counts": {"SUN": 4, "STAR": 5, "HEART": 6, "SMILE": 8, "MOON": 5, "NOTE": 5, "PLANET": 2, "BL": 2},
    "free_word_mode": True,
}


def render_board(board_json: str) -> None:
    board = json.loads(board_json)
    rows = board.get("rows", [])
    for row in rows:
        cells = [cell if cell else "·" for cell in row]
        print(" ".join(cells))


def main() -> None:
    game = Game(json.dumps(CONFIG), 2)
    # Seed rack so the move is deterministic.
    game.set_rack(["SUN", "HEART", "MOON", "STAR", "NOTE", "SMILE", "PLANET"])

    placements = [
        {"x": 2, "y": 3, "kind_id": "SUN"},
        {"x": 3, "y": 3, "kind_id": "HEART"},
        {"x": 4, "y": 3, "kind_id": "MOON"},
        {"x": 5, "y": 3, "kind_id": "STAR"},
    ]
    score = game.play_move(json.dumps(placements))
    print("Placed emoji word across the centre line: +{total} pts".format(**score))

    print("\nBoard after move:")
    render_board(game.get_board_json())

    print("\nRemaining rack:", game.get_rack_json())
    print("Snapshot extract:", game.snapshot_state_json()[:120], "...")

    print("\nEvent log:")
    for event in game.event_log():
        print(" ", event)
if __name__ == "__main__":
    main()
