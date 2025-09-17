extends Button

@export var kind_id: String = ""
@export var symbol: String = ""
@export var score: int = 0
@export var board: Node

func _get_drag_data(at_position: Vector2):
    var preview := Label.new()
    preview.text = "%s\n%d" % [symbol if symbol != "" else kind_id, score]
    preview.add_theme_color_override("font_color", Color.BLACK)
    set_drag_preview(preview)
    return {"kind_id": kind_id}

