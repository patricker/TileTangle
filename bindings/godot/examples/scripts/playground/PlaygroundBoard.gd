extends Control

class_name PlaygroundBoard

signal cell_pressed(x: int, y: int)
signal tile_dropped_on_cell(x: int, y: int, kind_id: String)

var layout := {}
var tile_lookup := {}
var bonus_map := {}

var cells := {}
var mask := {}
var hex_mode := false

var board_state := {}
var staged_state := {}
var highlight_state := {}

func configure(new_layout: Dictionary, tiles: Dictionary, bonuses: Array) -> void:
  layout = new_layout.duplicate(true)
  tile_lookup = tiles.duplicate(true)
  hex_mode = bool(layout.get("hex", false))
  mask = layout.get("mask", {}).duplicate(true)
  _build_bonus_map(bonuses)
  _rebuild_cells()

func update_board(state: Dictionary, staged: Dictionary, highlight: Dictionary) -> void:
  board_state = state
  staged_state = staged
  highlight_state = highlight
  _refresh_cells()

func clear_staged() -> void:
  staged_state.clear()
  highlight_state.erase("preview_main")
  highlight_state.erase("preview_cross")
  highlight_state.erase("invalid")
  _refresh_cells()

func _build_bonus_map(bonuses: Array) -> void:
  bonus_map.clear()
  for entry in bonuses:
    if entry is Dictionary and entry.has("x") and entry.has("y"):
      var key := _key(int(entry.x), int(entry.y))
      bonus_map[key] = entry

func _rebuild_cells() -> void:
  for child in get_children():
    child.queue_free()
  cells.clear()
  if layout.is_empty():
    return
  var width := int(layout.get("width", 0))
  var height := int(layout.get("height", 0))
  for y in range(height):
    for x in range(width):
      var key := _key(x, y)
      if not mask.has(key):
        continue
      var cell := PlaygroundBoardCell.new()
      cell.set_coord(x, y)
      cell.set_shape("hex" if hex_mode else "square")
      cell.set_bonus(bonus_map.get(key, {}))
      cell.connect("cell_clicked", Callable(self, "_on_cell_clicked"))
      cell.connect("tile_dropped", Callable(self, "_on_tile_dropped"))
      add_child(cell)
      cells[key] = cell
  _position_cells()
  _refresh_cells()

func _refresh_cells() -> void:
  for key in cells.keys():
    var cell := cells[key]
    if board_state.has(key):
      cell.apply_tile(board_state[key])
    else:
      cell.clear_tile()
    if staged_state.has(key):
      cell.stage_tile(staged_state[key])
    else:
      cell.clear_stage()
    cell.set_highlight(_highlight_for_key(key))

func _highlight_for_key(key: String) -> String:
  if highlight_state.get("invalid", false) and staged_state.has(key):
    return "invalid"
  if _in_collection(highlight_state.get("preview_main", []), key):
    return "preview_main"
  if _in_collection(highlight_state.get("preview_cross", []), key):
    return "preview_cross"
  if _in_collection(highlight_state.get("last_main", []), key):
    return "last_main"
  if _in_collection(highlight_state.get("last_cross", []), key):
    return "last_cross"
  return ""

func _in_collection(collection, key: String) -> bool:
  if collection is Dictionary:
    return collection.has(key)
  if collection is Array:
    for item in collection:
      if item is String and item == key:
        return true
      if item is Array and item.size() >= 2 and _key(int(item[0]), int(item[1])) == key:
        return true
  return false

func _notification(what: int) -> void:
  if what == NOTIFICATION_RESIZED:
    _position_cells()

func _position_cells() -> void:
  if cells.is_empty():
    return
  if hex_mode:
    _position_hex_cells()
  else:
    _position_grid_cells()

func _position_grid_cells() -> void:
  var width := float(layout.get("width", 0))
  var height := float(layout.get("height", 0))
  if width <= 0 or height <= 0:
    return
  var size := get_size()
  var cell := min(size.x / width, size.y / height)
  var board_w := cell * width
  var board_h := cell * height
  var origin := Vector2((size.x - board_w) * 0.5, (size.y - board_h) * 0.5)
  for key in cells.keys():
    var parts := key.split(",")
    var x := int(parts[0])
    var y := int(parts[1])
    var node: PlaygroundBoardCell = cells[key]
    node.position = origin + Vector2(x * cell, y * cell)
    node.size = Vector2(cell, cell)
    node.set_shape("square")

func _position_hex_cells() -> void:
  var width := float(layout.get("width", 0))
  var height := float(layout.get("height", 0))
  if width <= 0 or height <= 0:
    return
  var size := get_size()
  var sqrt3 := sqrt(3.0)
  var radius_w := size.x / (width * sqrt3 + sqrt3 * 0.5)
  var radius_h := size.y / (radius_height(height))
  var radius := min(radius_w, radius_h)
  var hex_w := sqrt3 * radius
  var hex_h := radius * 2.0
  var board_w := hex_w * width + (hex_w * 0.5)
  var board_h := hex_h + radius * 1.5 * max(height - 1.0, 0.0)
  var origin := Vector2((size.x - board_w) * 0.5, (size.y - board_h) * 0.5)
  for key in cells.keys():
    var parts := key.split(",")
    var x := int(parts[0])
    var y := int(parts[1])
    var node: PlaygroundBoardCell = cells[key]
    var offset := (y % 2) * (hex_w * 0.5)
    var center := Vector2(hex_w * x + offset + hex_w * 0.5, radius + 1.5 * radius * y)
    var top_left := origin + center - Vector2(hex_w * 0.5, hex_h * 0.5)
    node.position = top_left
    node.size = Vector2(hex_w, hex_h)
    node.set_shape("hex")

func radius_height(height: float) -> float:
  if height <= 1.0:
    return 2.0
  return 2.0 + 1.5 * (height - 1.0)

func _on_cell_clicked(x: int, y: int) -> void:
  emit_signal("cell_pressed", x, y)

func _on_tile_dropped(x: int, y: int, kind_id: String) -> void:
  emit_signal("tile_dropped_on_cell", x, y, kind_id)

static func _key(x: int, y: int) -> String:
  return "%d,%d" % [x, y]
