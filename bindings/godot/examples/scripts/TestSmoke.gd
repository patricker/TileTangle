extends Node

func run_tests():
    if not ClassDB.class_exists("WordEngine"):
        push_error("WordEngine missing")
        return
    var e = ClassDB.instantiate("WordEngine")
    var cfg = {
        "tileset": {"tile_kinds": [
            {"id": "A", "symbol": "A", "score": 1}
        ]},
        "rack_size": 7,
        "board_layout": {"width": 3, "height": 3},
        "ruleset_id": "cross",
        "dictionary_id": "en",
        "rng_seed": 1,
        "tile_counts": {"A": 10},
        "free_word_mode": true,
    }
    assert(e.new_game(JSON.stringify(cfg), 2))
    var score := e.play_move('[{"x":1,"y":1,"kind_id":"A"}]')
    assert(score != "")
    var board := e.get_board_json()
    assert(board != "")
    print("Godot smoke tests passed")
