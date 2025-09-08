import json

def test_import_and_basic_flow():
    import tiletangle
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
    g = tiletangle.Game(json.dumps(cfg), 2)
    board = json.loads(g.get_board_json())
    assert board["width"] == 5 and board["height"] == 5
    sc = g.play_move(json.dumps([
        {"x": 2, "y": 2, "kind_id": "A"},
        {"x": 3, "y": 2, "kind_id": "B"},
    ]))
    assert sc["total"] >= 0

