extends Button

@export var x: int
@export var y: int
@export var board: Node

func _can_drop_data(_pos, data):
	return typeof(data) == TYPE_DICTIONARY and data.has("kind_id")

func _drop_data(_pos, data):
	if board and data.has("kind_id"):
		board.stage_tile(x, y, String(data["kind_id"]))

func _gui_input(event):
	if event is InputEventMouseMotion and get_tree().is_dragging():
		modulate = Color(0.85, 1.0, 0.85)
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		modulate = Color.WHITE
