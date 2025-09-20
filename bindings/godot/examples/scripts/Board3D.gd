extends Node3D

var eng
var W := 5
var H := 5
var D := 3
var TILE := "A"
var cell_size := Vector3(1.2, 0.2, 1.2)
var layers: Array = []
var selected_layer := 0
var slice_height := 1

func _ready():
    eng = _instantiate_engine()
    if eng == null:
        push_error("WordEngine extension missing")
        return
    _init_engine()
    _build_grid()
    _refresh_symbols()

func _instantiate_engine():
    if not ClassDB.class_exists("WordEngine"):
        return null
    return ClassDB.instantiate("WordEngine")

func _init_engine():
    var cfg: Dictionary = {
        "tileset": {"tile_kinds": [
            {"id": "A", "symbol": "A", "score": 1},
            {"id": "B", "symbol": "B", "score": 3},
            {"id": "C", "symbol": "C", "score": 3}
        ]},
        "rack_size": 7,
        "ruleset_id": "cross",
        "dictionary_id": "en",
        "rng_seed": 42,
        "tile_counts": {"A": 40, "B": 40, "C": 40},
        "free_word_mode": true,
        "board_layout": {"type": "3d", "width": W, "height": H, "depth": D}
    }
    var ok: bool = eng.new_game(JSON.stringify(cfg), 2)
    if not ok:
        push_error("Failed to create game")

func _build_grid():
    # Create a simple stacked grid of boxes per layer
    var box := BoxMesh.new()
    box.size = Vector3(1, 0.1, 1)
    var mat := StandardMaterial3D.new()
    mat.albedo_color = Color(0.9, 0.9, 0.9)
    box.material = mat
    layers.clear()
    for z in range(D):
        var layer := Node3D.new()
        layer.name = "Layer_%d" % z
        layer.translation = Vector3(0, z * (cell_size.y + 0.2), 0)
        add_child(layer)
        layers.append(layer)
        for y in range(H):
            for x in range(W):
                var mi := MeshInstance3D.new()
                mi.mesh = box
                mi.translation = Vector3(x * cell_size.x, 0, y * cell_size.z)
                layer.add_child(mi)
    set_slice(selected_layer)

func _refresh_symbols():
    # Clear old labels
    for n in get_children():
        if n is Node3D:
            for c in n.get_children():
                if c is Label3D:
                    c.queue_free()
    # Read board and spawn Label3D for tiles
    var board_json: String = eng.get_board_json()
    if board_json == "":
        return
    var parsed := JSON.parse_string(board_json)
    if parsed == null:
        return
    var slice_h := max(1, int(parsed["height"]) / max(1, D))
    slice_height = slice_h
    for z in range(D):
        if z >= layers.size():
            continue
        var layer: Node3D = layers[z]
        for y in range(H):
            for x in range(W):
                var yy: int = y + z * slice_h
                var sym := String(parsed["rows"][yy][x])
                if sym.length() == 0:
                    continue
                var lbl := Label3D.new()
                lbl.text = sym
                lbl.billboard = BaseMaterial3D.BILLBOARD_DISABLED
                lbl.translation = Vector3(x * cell_size.x, 0.15, y * cell_size.z)
                layer.add_child(lbl)

func slice_count() -> int:
    return D

func current_slice() -> int:
    return selected_layer

func set_slice(z: int) -> void:
    if layers.is_empty():
        selected_layer = int(clamp(z, 0, max(0, D - 1)))
        return
    selected_layer = int(clamp(z, 0, layers.size() - 1))
    for i in range(layers.size()):
        layers[i].visible = (i == selected_layer)

func play_sample_word() -> void:
    var board_json: String = eng.get_board_json()
    if board_json == "":
        return
    var board = JSON.parse_string(board_json)
    if board == null:
        return
    var rack_json: String = eng.get_rack_json()
    if rack_json == "":
        return
    var rack = JSON.parse_string(rack_json)
    if rack == null:
        return
    var tiles := []
    for entry in rack:
        if tiles.size() >= D:
            break
        tiles.append(String(entry.get("kind_id", "A")))
    if tiles.size() < 3:
        push_warning("Need at least three tiles to play sample 3D word")
        return
    var full_h := int(board.get("height", H * D))
    var slice_h := max(1, full_h / max(1, D))
    var cx := int(W / 2)
    var cy := int(H / 2)
    var placements := []
    for i in range(min(tiles.size(), D)):
        placements.append({
            "x": cx,
            "y": cy + i * slice_h,
            "kind_id": tiles[i]
        })
    var placements_json := JSON.stringify(placements)
    var res: String = eng.play_move(placements_json)
    if res == "":
        push_error("Failed to apply sample 3D word")
    _refresh_symbols()
    set_slice(selected_layer)
