extends Node

class_name PlaygroundConfig

const BOARD_SHAPES := ["rect", "diamond", "triangle", "ring", "cross", "hexagon"]
const ADJACENCY_MODES := ["orthogonal", "diagonal", "hex"]

static func classic_tileset() -> Dictionary:
  var entries := [
    {"id": "A", "symbol": "A", "score": 1, "count": 9},
    {"id": "B", "symbol": "B", "score": 3, "count": 2},
    {"id": "C", "symbol": "C", "score": 3, "count": 2},
    {"id": "D", "symbol": "D", "score": 2, "count": 4},
    {"id": "E", "symbol": "E", "score": 1, "count": 12},
    {"id": "F", "symbol": "F", "score": 4, "count": 2},
    {"id": "G", "symbol": "G", "score": 2, "count": 3},
    {"id": "H", "symbol": "H", "score": 4, "count": 2},
    {"id": "I", "symbol": "I", "score": 1, "count": 9},
    {"id": "J", "symbol": "J", "score": 8, "count": 1},
    {"id": "K", "symbol": "K", "score": 5, "count": 1},
    {"id": "L", "symbol": "L", "score": 1, "count": 4},
    {"id": "M", "symbol": "M", "score": 3, "count": 2},
    {"id": "N", "symbol": "N", "score": 1, "count": 6},
    {"id": "O", "symbol": "O", "score": 1, "count": 8},
    {"id": "P", "symbol": "P", "score": 3, "count": 2},
    {"id": "Q", "symbol": "Q", "score": 10, "count": 1},
    {"id": "R", "symbol": "R", "score": 1, "count": 6},
    {"id": "S", "symbol": "S", "score": 1, "count": 4},
    {"id": "T", "symbol": "T", "score": 1, "count": 6},
    {"id": "U", "symbol": "U", "score": 1, "count": 4},
    {"id": "V", "symbol": "V", "score": 4, "count": 2},
    {"id": "W", "symbol": "W", "score": 4, "count": 2},
    {"id": "X", "symbol": "X", "score": 8, "count": 1},
    {"id": "Y", "symbol": "Y", "score": 4, "count": 2},
    {"id": "Z", "symbol": "Z", "score": 10, "count": 1},
    {"id": "BL", "symbol": "_", "score": 0, "count": 2, "is_blank": true},
  ]
  var tile_counts := {}
  var tile_kinds := []
  for entry in entries:
    tile_counts[entry.id] = entry.count
    var kind := {
      "id": entry.id,
      "symbol": entry.symbol,
      "score": entry.score,
    }
    if entry.has("is_blank") and entry.is_blank:
      kind["is_blank"] = true
    tile_kinds.append(kind)
  return {
    "tile_kinds": tile_kinds,
    "tile_counts": tile_counts,
  }

static func presets() -> Dictionary:
  return {
    "classic": {
      "label": "Classic 15×15",
      "width": 15,
      "height": 15,
      "rack": 7,
      "shape": "rect",
      "adjacency": "orthogonal",
      "bonus": "classic",
    },
    "hex_garden": {
      "label": "Hex Garden",
      "width": 13,
      "height": 13,
      "rack": 7,
      "shape": "diamond",
      "adjacency": "hex",
      "bonus": "hex",
    },
    "sprint": {
      "label": "Sprint 11×11",
      "width": 11,
      "height": 11,
      "rack": 6,
      "shape": "rect",
      "adjacency": "orthogonal",
      "bonus": "classic",
    },
    "triangle_peak": {
      "label": "Triangle Peak",
      "width": 13,
      "height": 13,
      "rack": 7,
      "shape": "triangle",
      "adjacency": "orthogonal",
      "bonus": "triangle",
    },
  }

static func preset_options() -> Array:
  var out := []
  for key in presets().keys():
    var data := presets()[key]
    out.append({"value": key, "label": data.label})
  return out

static func resolve_preset(key: String) -> Dictionary:
  var data := presets().get(key, null)
  if data == null:
    return presets()["classic"]
  return data.duplicate(true)

static func build_mask(width: int, height: int, shape: String) -> Dictionary:
  var mask := {}
  var mid_x := (width - 1) / 2.0
  var mid_y := (height - 1) / 2.0
  var diamond_radius := int(min(width, height) / 2)
  for y in range(height):
    for x in range(width):
      var key := _key(x, y)
      var include := false
      match shape:
        "rect":
          include = true
        "diamond":
          include = abs(x - mid_x) + abs(y - mid_y) <= diamond_radius
        "cross":
          include = int(round(mid_x)) == x or int(round(mid_y)) == y
        "hexagon":
          var dx := x - mid_x
          var dy := y - mid_y
          include = abs(dx) + abs(dy) + abs(dx + dy) <= int(min(width, height) * 1.5)
        "triangle":
          var rel_y := y - int(min(mid_y, mid_x))
          if rel_y >= 0:
            var span := width - rel_y * 2
            if span > 0:
              var left := int(floor((width - span) / 2.0))
              include = x >= left and x < left + span
        "ring":
          var dx2 := x - mid_x
          var dy2 := y - mid_y
          var dist := sqrt(dx2 * dx2 + dy2 * dy2)
          var outer := min(mid_x, mid_y) + 0.5
          var inner := max(outer - 2.0, 0.0)
          include = dist <= outer and dist >= inner
        _:
          include = true
      if include:
        mask[key] = true
  return mask

static func build_overlay(width: int, height: int, mask: Dictionary, adjacency: String) -> Dictionary:
  var nodes := []
  var index_for := {}
  var idx := 0
  for y in range(height):
    for x in range(width):
      var key := _key(x, y)
      if mask.has(key):
        nodes.append({"x": x, "y": y})
        index_for[key] = idx
        idx += 1
  var edges := []
  var add_edge = func(ax: int, ay: int, bx: int, by: int, dir: String) -> void:
    var key_a := _key(ax, ay)
    var key_b := _key(bx, by)
    if not index_for.has(key_a) or not index_for.has(key_b):
      return
    edges.append({
      "a": index_for[key_a],
      "b": index_for[key_b],
      "dir": dir,
    })
  for y in range(height):
    for x in range(width):
      if not mask.has(_key(x, y)):
        continue
      if adjacency == "orthogonal" or adjacency == "diagonal":
        add_edge(x, y, x + 1, y, "E")
        add_edge(x, y, x, y + 1, "S")
        if adjacency == "diagonal":
          add_edge(x, y, x + 1, y + 1, "SE")
          add_edge(x, y, x - 1, y + 1, "SW")
      elif adjacency == "hex":
        add_edge(x, y, x + 1, y, "E")
        var even := (y % 2) == 0
        var up_x := x + (0 if even else 1)
        var down_x := x + (0 if even else 1)
        add_edge(x, y, up_x, y - 1, "NE")
        add_edge(x, y, down_x, y + 1, "SE")
      else:
        add_edge(x, y, x + 1, y, "E")
        add_edge(x, y, x, y + 1, "S")
  return {"nodes": nodes, "edges": edges}

static func build_bonus_layout(width: int, height: int, shape: String, adjacency: String, preset: String) -> Array:
  var kind := _resolve_bonus_kind(preset, shape, adjacency)
  if kind == "none":
    return []
  var map := _build_bonus_map(kind, width, height)
  var out := []
  for cell in map.values():
    if cell.x >= 0 and cell.x < width and cell.y >= 0 and cell.y < height:
      out.append(cell)
  return out

static func build_config(settings: Dictionary) -> Dictionary:
  var preset_data := resolve_preset(settings.get("preset", "classic"))
  var width := int(settings.get("width", preset_data.width))
  var height := int(settings.get("height", preset_data.height))
  width = max(width, 3)
  height = max(height, 3)
  var rack := int(settings.get("rack_size", preset_data.rack))
  var shape := String(settings.get("shape", preset_data.shape))
  var adjacency := String(settings.get("adjacency", preset_data.adjacency))
  if not BOARD_SHAPES.has(shape):
    shape = "rect"
  if not ADJACENCY_MODES.has(adjacency):
    adjacency = "orthogonal"
  var bonus_preset := String(settings.get("bonus", preset_data.bonus))
  var tileset := settings.get("tileset", classic_tileset())
  var mask := build_mask(width, height, shape)
  var overlay := build_overlay(width, height, mask, adjacency)
  var bonuses := build_bonus_layout(width, height, shape, adjacency, bonus_preset)
  var board_layout := {"width": width, "height": height}
  if overlay.nodes.size() > 0:
    board_layout["type"] = "graph"
    board_layout["nodes"] = overlay.nodes
    board_layout["edges"] = overlay.edges
  var config := {
    "tileset": {"tile_kinds": tileset.tile_kinds},
    "rack_size": rack,
    "board_layout": board_layout,
    "ruleset_id": settings.get("ruleset_id", "cross"),
    "dictionary_id": settings.get("dictionary_id", "en"),
    "rng_seed": int(settings.get("rng_seed", 1)),
    "tile_counts": tileset.tile_counts,
    "free_word_mode": settings.get("free_word_mode", true),
  }
  return {
    "config": config,
    "bonuses": bonuses,
    "layout": {
      "width": width,
      "height": height,
      "mask": mask,
      "hex": adjacency == "hex",
      "shape": shape,
      "adjacency": adjacency,
    },
    "tileset": tileset,
  }

static func _resolve_bonus_kind(preset: String, shape: String, adjacency: String) -> String:
  match preset:
    "none":
      return "none"
    "classic":
      return "classic"
    "hex":
      return "hex"
    "triangle":
      return "triangle"
    "ring":
      return "ring"
    _:
      if adjacency == "hex" or shape == "diamond" or shape == "hexagon":
        return "hex"
      if shape == "triangle":
        return "triangle"
      if shape == "ring":
        return "ring"
      return "classic"

static func _build_bonus_map(kind: String, width: int, height: int) -> Dictionary:
  var map := _baseline_bonus(width, height)
  match kind:
    "hex":
      _apply_hex_accents(map, width, height)
    "triangle":
      _apply_triangle_accents(map, width, height)
    "ring":
      _apply_ring_accents(map, width, height)
    _:
      pass
  return map

static func _baseline_bonus(width: int, height: int) -> Dictionary:
  var map := {}
  var center_x := (width - 1) / 2.0
  var center_y := (height - 1) / 2.0
  for y in range(height):
    for x in range(width):
      var edge := min(min(x, width - 1 - x), min(y, height - 1 - y))
      if edge == 0:
        _set_word(map, x, y, 3, [])
        continue
      if edge == 1:
        _set_word(map, x, y, 2, [])
        continue
      var manhattan := abs(x - center_x) + abs(y - center_y)
      if manhattan == 0:
        continue
      if int(manhattan) % 4 == 0:
        _set_letter(map, x, y, 3, [])
      elif (x + y) % 3 == 0:
        _set_letter(map, x, y, 2, [])
  var mid_x := int(round(center_x))
  var mid_y := int(round(center_y))
  if mid_x >= 0 and mid_x < width and mid_y >= 0 and mid_y < height:
    _set_word(map, mid_x, mid_y, 2, ["center"])
  return map

static func _apply_hex_accents(map: Dictionary, width: int, height: int) -> void:
  var cx := (width - 1) / 2.0
  var cy := (height - 1) / 2.0
  for y in range(height):
    for x in range(width):
      var axial_q := x - cx
      var axial_r := y - cy
      var axial_s := -axial_q - axial_r
      var radius := max(abs(axial_q), max(abs(axial_r), abs(axial_s)))
      if radius == 2:
        _set_letter(map, x, y, 3, ["hex"])
      elif radius == 3:
        _set_letter(map, x, y, 2, ["hex"])

static func _apply_triangle_accents(map: Dictionary, width: int, height: int) -> void:
  for y in range(height):
    for x in range(width):
      var diag := x - y
      if abs(diag) == 0:
        _set_letter(map, x, y, 3, ["diag"])
      elif abs(diag) == 1:
        _set_letter(map, x, y, 2, ["diag"])

static func _apply_ring_accents(map: Dictionary, width: int, height: int) -> void:
  var cx := (width - 1) / 2.0
  var cy := (height - 1) / 2.0
  for y in range(height):
    for x in range(width):
      var dist := max(abs(x - cx), abs(y - cy))
      if dist == 2:
        _set_word(map, x, y, 3, ["ring"])
      elif dist == 3:
        _set_word(map, x, y, 2, ["ring"])

static func _set_word(map: Dictionary, x: int, y: int, mul: int, tags: Array) -> void:
  var key := _key(x, y)
  if map.has(key) and map[key].has("word_mul") and map[key].word_mul >= mul:
    return
  map[key] = {"x": x, "y": y, "word_mul": mul, "tags": tags.duplicate()}

static func _set_letter(map: Dictionary, x: int, y: int, mul: int, tags: Array) -> void:
  var key := _key(x, y)
  if map.has(key):
    var entry := map[key]
    if entry.has("word_mul"):
      return
    if entry.has("letter_mul") and entry.letter_mul >= mul:
      return
  map[key] = {"x": x, "y": y, "letter_mul": mul, "tags": tags.duplicate()}

static func _key(x: int, y: int) -> String:
  return "%d,%d" % [x, y]
