extends Node3D

var eng := WordEngine.new()
var W := 5
var H := 5
var D := 3
var TILE := "A"
var cell_size := Vector3(1.2, 0.2, 1.2)

func _ready():
    _init_engine()
    _build_grid()
    _refresh_symbols()

func _init_engine():
    var cfg = {
        "tileset": {"tile_kinds": [
            {"id": "A", "symbol": "A", "score": 1},
            {"id": "B", "symbol": "B", "score": 3}
        ]},
        "rack_size": 7,
        "ruleset_id": "cross",
        "dictionary_id": "en",
        "rng_seed": 42,
        "tile_counts": {"A": 50, "B": 50},
        "free_word_mode": true,
        "board_layout": {"type": "3d", "width": W, "height": H, "depth": D}
    }
    var ok = eng.new_game(JSON.stringify(cfg), 2)
    if not ok:
        push_error("Failed to create game")

func _build_grid():
    # Create a simple stacked grid of boxes per layer
    var box := BoxMesh.new()
    box.size = Vector3(1, 0.1, 1)
    var mat := StandardMaterial3D.new()
    mat.albedo_color = Color(0.9, 0.9, 0.9)
    box.material = mat
    for z in D:
        var layer := Node3D.new()
        layer.name = "Layer_%d" % z
        layer.translation = Vector3(0, z * (cell_size.y + 0.2), 0)
        add_child(layer)
        for y in H:
            for x in W:
                var mi := MeshInstance3D.new()
                mi.mesh = box
                mi.translation = Vector3(x * cell_size.x, 0, y * cell_size.z)
                layer.add_child(mi)

func _refresh_symbols():
    # Clear old labels
    for n in get_children():
        if n is Node3D:
            for c in n.get_children():
                if c is Label3D:
                    c.queue_free()
    # Read board and spawn Label3D for tiles
    var board_json = eng.get_board_json()
    if board_json == "":
        return
    var parsed = JSON.parse_string(board_json)
    if parsed == null:
        return
    var slice_h := int(parsed["height"]) / D
    for z in D:
        var layer := get_node("Layer_%d" % z)
        for y in H:
            for x in W:
                var yy := y + z * slice_h
                var sym := String(parsed["rows"][yy][x])
                if sym.length() == 0:
                    continue
                var lbl := Label3D.new()
                lbl.text = sym
                lbl.billboard = BaseMaterial3D.BILLBOARD_DISABLED
                lbl.translation = Vector3(x * cell_size.x, 0.15, y * cell_size.z)
                layer.add_child(lbl)

