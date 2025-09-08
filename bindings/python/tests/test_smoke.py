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

def test_error_path_invalid_coordinates():
    import tiletangle
    cfg = json.loads(open("test_data/configs/config_small.json").read())
    g = tiletangle.Game(json.dumps(cfg), 2)
    try:
        g.play_move(json.dumps([
            {"x": -1, "y": 0, "kind_id": "A"},
        ]))
        assert False, "should have raised"
    except Exception as e:
        assert "invalid coordinates" in str(e).lower()

def test_conformance_board_after_move():
    import tiletangle
    cfg = open("test_data/configs/config_small.json").read()
    mv = open("test_data/moves/move_ab.json").read()
    expect = json.loads(open("test_data/expect/board_after_ab.json").read())
    g = tiletangle.Game(cfg, 2)
    g.play_move(mv)
    board = json.loads(g.get_board_json())
    assert board == expect
