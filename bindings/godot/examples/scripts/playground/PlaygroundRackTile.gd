extends Button

class_name PlaygroundRackTile

@export var kind_id := ""
@export var symbol := ""
@export var score := 0

var selected := false

func set_tile(info: Dictionary) -> void:
  kind_id = String(info.get("kind_id", ""))
  symbol = String(info.get("symbol", kind_id))
  score = int(info.get("score", 0))
  text = "%s\n%d" % [symbol, score]

func select(active: bool) -> void:
  selected = active
  modulate = Color(0.92, 0.9, 0.6) if active else Color(1, 1, 1)

func _get_drag_data(_position: Vector2):
  var preview := Label.new()
  preview.text = "%s\n%d" % [symbol, score]
  preview.add_theme_color_override("font_color", Color(0.05, 0.05, 0.05))
  set_drag_preview(preview)
  return {"kind_id": kind_id}
