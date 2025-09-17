extends Control

@onready var grid := GridContainer.new()
var eng := WordEngine.new()
var width := 5
var height := 5
var use_hex := true
var selected_kind_id := ""
var staged := [] # array of {x,y,kind_id}
var kind_to_symbol := {}
var last_main := []
var last_cross := []

func _ready():
    # Top bar with toggles
    var bar := HBoxContainer.new()
    bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    add_child(bar)

    var btn_free := Button.new()
    btn_free.text = "Free Word Mode"
    btn_free.toggle_mode = true
    btn_free.button_pressed = true
    btn_free.pressed.connect(func():
        eng.set_free_word_mode(btn_free.button_pressed)
    )
    bar.add_child(btn_free)

    var btn_hex := Button.new()
    btn_hex.text = "Hex Geometry"
    btn_hex.toggle_mode = true
    btn_hex.button_pressed = use_hex
    btn_hex.pressed.connect(func():
        use_hex = btn_hex.button_pressed
        _new_game_and_rebuild_grid()
    )
    bar.add_child(btn_hex)

    # Score label
    var score_label := Label.new()
    score_label.name = "ScoreLabel"
    score_label.text = ""
    add_child(score_label)

    # Commit / Cancel buttons
    var btn_commit := Button.new()
    btn_commit.text = "Commit Move"
    btn_commit.pressed.connect(func(): _commit_staged())
    bar.add_child(btn_commit)

    var btn_cancel := Button.new()
    btn_cancel.text = "Cancel"
    btn_cancel.pressed.connect(func(): _clear_staged())
    bar.add_child(btn_cancel)

    add_child(grid)
    grid.columns = width
    grid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    grid.size_flags_vertical = Control.SIZE_EXPAND_FILL

    _new_game_and_rebuild_grid()
    _refresh_rack()

func _new_game_and_rebuild_grid():
    # Clear grid
    while grid.get_child_count() > 0:
        var c = grid.get_child(0)
        grid.remove_child(c)
        c.queue_free()

    # Build config
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
    if use_hex:
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

    # Build grid buttons
    for y in height:
        for x in width:
            var btn := Button.new()
            btn.text = ""
            btn.pressed.connect(func(): _on_cell_pressed(x, y))
            var sc := preload("res://scripts/BoardCell.gd")
            btn.set_script(sc)
            btn.set("x", x)
            btn.set("y", y)
            btn.set("board", self)
            grid.add_child(btn)

    _refresh_board()

func _on_cell_pressed(x:int, y:int):
    var kid := selected_kind_id if selected_kind_id != "" else "A"
    var placements = [{"x": x, "y": y, "kind_id": kid}]
    var res = eng.play_move(JSON.stringify(placements))
    if res == "":
        push_error("play_move failed")
    else:
        _update_score_overlay(res)
    _refresh_board()
    _refresh_rack()

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
                node.modulate = Color.WHITE
            idx += 1

    # Overlay staged placements
    for it in staged:
        var sx := int(it["x"])
        var sy := int(it["y"])
        var sym := kind_to_symbol.get(String(it["kind_id"]), String(it["kind_id"]))
        var i := sy * width + sx
        if i >= 0 and i < grid.get_child_count():
            var node := grid.get_child(i)
            if node is Button:
                node.text = sym
                # keep color neutral; highlights come from preview

    # Overlay preview highlights if staged; else show last commit highlights
    if staged.size() > 0:
        var preview := _preview_json()
        if preview != null:
            _apply_highlights(preview)
    elif last_main.size() > 0 || last_cross.size() > 0:
        _apply_highlight_vectors(last_main, last_cross)

func _apply_highlights(preview):
    if preview == null:
        return
    if preview.has("main_cells"):
        for c in preview["main_cells"]:
            var x := int(c[0]); var y := int(c[1])
            var i := y*width + x
            if i>=0 and i<grid.get_child_count():
                var node := grid.get_child(i)
                if node is Button: node.modulate = Color(1.0, 0.95, 0.7)
    if preview.has("cross_cells"):
        for arr in preview["cross_cells"]:
            for c in arr:
                var x := int(c[0]); var y := int(c[1])
                var i := y*width + x
                if i>=0 and i<grid.get_child_count():
                    var node := grid.get_child(i)
                    if node is Button: node.modulate = Color(0.95, 1.0, 0.8)

func _apply_highlight_vectors(main_arr, cross_arr):
    for c in main_arr:
        var x := int(c[0]); var y := int(c[1])
        var i := y*width + x
        if i>=0 and i<grid.get_child_count():
            var node := grid.get_child(i)
            if node is Button: node.modulate = Color(1.0, 0.95, 0.7)
    for arr in cross_arr:
        for c in arr:
            var x := int(c[0]); var y := int(c[1])
            var i := y*width + x
            if i>=0 and i<grid.get_child_count():
                var node := grid.get_child(i)
                if node is Button: node.modulate = Color(0.95, 1.0, 0.8)

func _preview_json():
    if staged.is_empty():
        return null
    var res := eng.preview_move(JSON.stringify(staged))
    if res == "":
        return null
    var parsed := JSON.parse_string(res)
    if parsed == null:
        return null
    var total := int(parsed.get("total", 0))
    var main_word := String(parsed.get("main_word", ""))
    var main_score := int(parsed.get("main_score", 0))
    var cross_words := parsed.get("cross_words", [])
    var cross_sum := 0
    for cw in cross_words:
        if cw.size() >= 2:
            cross_sum += int(cw[1])
    var label := get_node("ScoreLabel") as Label
    if label:
        var valid := bool(parsed.get("valid", false))
        label.text = valid ? "Preview: %s total=%d (main=%d +cross=%d)" % [main_word, total, main_score, cross_sum] : "Preview: invalid"
    return parsed
func _refresh_rack():
    # Remove existing rack bar if present
    var old := get_node_or_null("RackBar")
    if old:
        remove_child(old)
        old.queue_free()
    var rack_json := eng.get_rack_json()
    if rack_json == "":
        return
    var parsed = JSON.parse_string(rack_json)
    if parsed == null:
        return
    var bar := HBoxContainer.new()
    bar.name = "RackBar"
    add_child(bar)
    kind_to_symbol.clear()
    for item in parsed:
        var btn := Button.new()
        var label := item["symbol"] if item.has("symbol") else item["kind_id"]
        var score := item.get("score", 0)
        btn.text = "%s\n%d" % [label, score]
        var kid := item["kind_id"]
        kind_to_symbol[String(kid)] = String(label)
        btn.pressed.connect(func():
            selected_kind_id = kid
            _highlight_selected_rack_button(btn)
        )
        var rs := preload("res://scripts/RackTile.gd")
        btn.set_script(rs)
        btn.set("kind_id", String(kid))
        btn.set("symbol", String(label))
        btn.set("score", int(score))
        btn.set("board", self)
        bar.add_child(btn)

func _highlight_selected_rack_button(sel_btn: Button):
    var bar := get_node_or_null("RackBar")
    if bar == null:
        return
    for c in bar.get_children():
        if c is Button:
            c.modulate = Color.WHITE
    sel_btn.modulate = Color(0.95, 0.95, 0.6)

func _update_score_overlay(score_json: String):
    var parsed = JSON.parse_string(score_json)
    if parsed == null:
        return
    var total := int(parsed.get("total", 0))
    var main_word := String(parsed.get("main_word", ""))
    var main_score := int(parsed.get("main_score", 0))
    var cross_words := parsed.get("cross_words", [])
    var cross_sum := 0
    for cw in cross_words:
        if cw.size() >= 2:
            cross_sum += int(cw[1])
    var label := get_node("ScoreLabel") as Label
    if label:
        label.text = "Last: %s total=%d (main=%d +cross=%d)" % [main_word, total, main_score, cross_sum]

func _commit_staged():
    if staged.is_empty():
        return
    # compute highlights first and cache
    var preview := _preview_json()
    if preview != null:
        last_main = preview.get("main_cells", [])
        last_cross = preview.get("cross_cells", [])
    var res = eng.play_move(JSON.stringify(staged))
    if res == "":
        push_error("play_move failed")
    else:
        _update_score_overlay(res)
    staged.clear()
    _refresh_board()
    _refresh_rack()

func _clear_staged():
    staged.clear()
    _refresh_board()

func stage_tile(x:int, y:int, kid:String):
    var new_staged := []
    var replaced := false
    for it in staged:
        if int(it["x"]) == x and int(it["y"]) == y:
            new_staged.append({"x": x, "y": y, "kind_id": kid})
            replaced = true
        else:
            new_staged.append(it)
    if not replaced:
        new_staged.append({"x": x, "y": y, "kind_id": kid})
    staged = new_staged
    _refresh_board()
