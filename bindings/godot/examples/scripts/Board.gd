extends Control

@onready var grid := GridContainer.new()
var eng := WordEngine.new()
var width := 5
var height := 5
var USE_HEX := true

func _ready():
    add_child(grid)
    grid.columns = width
    grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    grid.size_flags_vertical = Control.SIZE_EXPAND_FILL

    var base = {
        "tileset": {"tile_kinds": [
            {"id": "A", "symbol": "A", "score": 1},
            {"id": "B", "symbol": "B", "score": 3}
        ]},
        "rack_size": 7,
        "ruleset_id": "cross",
        "dictionary_id": "en",
        "rng_seed": 42,
        "tile_counts": {"A": 10, "B": 10},
        "free_word_mode": true,
    }
    var cfg := base.duplicate(true)
    if USE_HEX:
        var nodes := []
        for y in height:
            for x in width:
                nodes.append({"x": x, "y": y})
        func idx(x:int,y:int)->int: return y*width + x
        var edges := []
        func try_edge(x1:int,y1:int,x2:int,y2:int,dir:String):
            if x2<0 or x2>=width or y2<0 or y2>=height: return
            edges.append({"a": idx(x1,y1), "b": idx(x2,y2), "dir": dir})
        for y in height:
            for x in width:
                var even := (y % 2) == 0
                try_edge(x,y,x+1,y,"E")
                try_edge(x,y,x + (even?0:1), y-1, "NE")
                try_edge(x,y,x + (even?0:1), y+1, "SE")
        cfg["board_layout"] = {"width": width, "height": height, "type":"graph", "nodes": nodes, "edges": edges}
    else:
        cfg["board_layout"] = {"width": width, "height": height}
    var ok = eng.new_game(JSON.stringify(cfg), 2)
    if not ok:
        push_error("Failed to create game")
        return

    for y in height:
        for x in width:
            var btn := Button.new()
            btn.text = ""
            btn.pressed.connect(func(): _on_cell_pressed(x, y))
            grid.add_child(btn)

    _refresh_board()

func _on_cell_pressed(x:int, y:int):
    var placements = [{"x": x, "y": y, "kind_id": "A"}]
    var res = eng.play_move(JSON.stringify(placements))
    if res == "":
        push_error("play_move failed")
    _refresh_board()

func _refresh_board():
    var board_json = eng.get_board_json()
    if board_json == "":
        return
    var parsed = JSON.parse_string(board_json)
    if parsed == null:
        return
    var idx := 0
    for y in height:
        for x in width:
            var node := grid.get_child(idx)
            if node is Button:
                var symbol := parsed["rows"][y][x]
                node.text = symbol
            idx += 1
