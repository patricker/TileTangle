extends Control

@onready var board := get_node("../Board3D")
@onready var slider := $VBox/LayerSlider
@onready var layer_label := $VBox/LayerLabel

func _ready():
    if board == null:
        push_warning("Board3DOverlay: Board3D node missing")
        return
    slider.min_value = 0
    slider.max_value = max(0, board.slice_count() - 1)
    slider.step = 1
    slider.value = board.current_slice()
    layer_label.text = "Layer %d" % int(slider.value)

func _on_layer_slider_value_changed(value):
    if board:
        board.set_slice(int(value))
    layer_label.text = "Layer %d" % int(value)

func _on_play_button_pressed():
    if board:
        board.play_sample_word()
    layer_label.text = "Layer %d" % int(slider.value)
