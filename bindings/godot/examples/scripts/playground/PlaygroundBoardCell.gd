extends Control

class_name PlaygroundBoardCell

signal cell_clicked(x: int, y: int)
signal tile_dropped(x: int, y: int, kind_id: String)

const BASE_BG := Color(0.94, 0.93, 0.9, 1.0)
const BORDER_COLOR := Color(0.25, 0.26, 0.29, 1.0)
const STAGED_COLOR := Color(0.35, 0.6, 0.86, 0.35)
const PREVIEW_MAIN := Color(1.0, 0.92, 0.7, 0.65)
const PREVIEW_CROSS := Color(0.92, 1.0, 0.78, 0.65)
const INVALID_HIGHLIGHT := Color(0.93, 0.32, 0.32, 0.5)

var coord: Vector2i
var shape := "square"
var polygon := PackedVector2Array()

var bonus_label_text := ""
var bonus_color := BASE_BG

var base_tile := {}
var staged_tile := {}

var highlight_mode := ""
var hover := false

var tile_label: Label
var score_label: Label
var bonus_label: Label

func _ready() -> void:
  focus_mode = Control.FOCUS_NONE
  mouse_filter = Control.MOUSE_FILTER_STOP
  _ensure_labels()
  _rebuild_polygon()

func set_coord(x: int, y: int) -> void:
  coord = Vector2i(x, y)

func set_shape(value: String) -> void:
  shape = value
  _rebuild_polygon()
  queue_redraw()

func set_bonus(info: Dictionary) -> void:
  if info.is_empty():
    bonus_label_text = ""
    bonus_color = BASE_BG
  else:
    var text := ""
    if info.has("word_mul") and info.word_mul > 1:
      text = "%dW" % info.word_mul
    elif info.has("letter_mul") and info.letter_mul > 1:
      text = "%dL" % info.letter_mul
    else:
      text = ""
    bonus_label_text = text
    bonus_color = _color_for_bonus(info)
  _update_labels()
  queue_redraw()

func apply_tile(tile: Dictionary) -> void:
  base_tile = tile.duplicate(true)
  _update_labels()
  queue_redraw()

func clear_tile() -> void:
  base_tile.clear()
  _update_labels()
  queue_redraw()

func stage_tile(tile: Dictionary) -> void:
  staged_tile = tile.duplicate(true)
  _update_labels()
  queue_redraw()

func clear_stage() -> void:
  staged_tile.clear()
  _update_labels()
  queue_redraw()

func set_highlight(mode: String) -> void:
  highlight_mode = mode
  queue_redraw()

func _ensure_labels() -> void:
  if tile_label:
    return
  tile_label = Label.new()
  tile_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
  tile_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
  tile_label.add_theme_font_size_override("font_size", 26)
  add_child(tile_label)

  score_label = Label.new()
  score_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
  score_label.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM
  score_label.add_theme_font_size_override("font_size", 14)
  add_child(score_label)

  bonus_label = Label.new()
  bonus_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
  bonus_label.vertical_alignment = VERTICAL_ALIGNMENT_TOP
  bonus_label.add_theme_font_size_override("font_size", 13)
  bonus_label.modulate = Color(0.2, 0.2, 0.2, 0.9)
  add_child(bonus_label)

  _update_label_positions()
  _update_labels()

func _update_labels() -> void:
  var display := staged_tile if not staged_tile.is_empty() else base_tile
  if display.is_empty():
    tile_label.text = ""
    score_label.text = ""
  else:
    tile_label.text = String(display.get("symbol", display.get("kind_id", "")))
    score_label.text = str(display.get("score", 0))
  score_label.visible = score_label.text != ""
  bonus_label.text = bonus_label_text
  bonus_label.visible = bonus_label_text != ""
  if not staged_tile.is_empty():
    tile_label.modulate = Color(0.05, 0.1, 0.2, 1.0)
  else:
    tile_label.modulate = Color(0.1, 0.12, 0.15, 1.0)

func _notification(what: int) -> void:
  if what == NOTIFICATION_RESIZED:
    _rebuild_polygon()
    _update_label_positions()
    queue_redraw()

func _rebuild_polygon() -> void:
  var size := get_size()
  if shape == "hex":
    polygon = PackedVector2Array([
      Vector2(size.x * 0.5, 0.0),
      Vector2(size.x, size.y * 0.25),
      Vector2(size.x, size.y * 0.75),
      Vector2(size.x * 0.5, size.y),
      Vector2(0.0, size.y * 0.75),
      Vector2(0.0, size.y * 0.25),
    ])
  else:
    polygon = PackedVector2Array([
      Vector2.ZERO,
      Vector2(size.x, 0.0),
      Vector2(size.x, size.y),
      Vector2(0.0, size.y),
    ])

func _update_label_positions() -> void:
  var size := get_size()
  if not tile_label:
    return
  tile_label.size = size
  tile_label.position = Vector2.ZERO
  score_label.position = Vector2(size.x - 32.0, size.y - 28.0)
  score_label.size = Vector2(30.0, 26.0)
  bonus_label.position = Vector2(0.0, 2.0)
  bonus_label.size = Vector2(size.x, 18.0)

func _draw() -> void:
  if polygon.size() == 0:
    _rebuild_polygon()
  var color := bonus_color
  if not staged_tile.is_empty():
    color = color.lerp(STAGED_COLOR, 0.6)
  match highlight_mode:
    "preview_main":
      color = color.lerp(PREVIEW_MAIN, 0.7)
    "preview_cross":
      color = color.lerp(PREVIEW_CROSS, 0.7)
    "last_main":
      color = color.lerp(Color(0.98, 0.86, 0.65, 1.0), 0.6)
    "last_cross":
      color = color.lerp(Color(0.86, 0.96, 0.68, 1.0), 0.6)
    "invalid":
      color = color.lerp(INVALID_HIGHLIGHT, 0.8)
    _:
      pass
  if hover:
    color = color.lightened(0.08)
  draw_colored_polygon(polygon, color)
  draw_polyline(polygon, BORDER_COLOR, 1.6, true)

func _gui_input(event: InputEvent) -> void:
  if event is InputEventMouseMotion:
    hover = _point_inside(event.position)
    queue_redraw()
  elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
    if _point_inside(event.position):
      emit_signal("cell_clicked", coord.x, coord.y)

func _point_inside(pos: Vector2) -> bool:
  if polygon.size() == 0:
    return false
  return Geometry2D.is_point_in_polygon(pos, polygon)

func _can_drop_data(_pos: Vector2, data) -> bool:
  return typeof(data) == TYPE_DICTIONARY and data.has("kind_id")

func _drop_data(_pos: Vector2, data) -> void:
  if typeof(data) == TYPE_DICTIONARY and data.has("kind_id"):
    emit_signal("tile_dropped", coord.x, coord.y, String(data["kind_id"]))

func _color_for_bonus(info: Dictionary) -> Color:
  if info.has("word_mul"):
    match int(info.word_mul):
      3:
        return Color(0.93, 0.55, 0.58, 1.0)
      2:
        return Color(0.96, 0.74, 0.74, 1.0)
    return Color(0.9, 0.8, 0.8, 1.0)
  if info.has("letter_mul"):
    match int(info.letter_mul):
      3:
        return Color(0.53, 0.74, 0.91, 1.0)
      2:
        return Color(0.67, 0.84, 0.95, 1.0)
    return Color(0.82, 0.88, 0.95, 1.0)
  return BASE_BG
