extends Control
const MAX_EVENT_LOG := 20
var eng
var rng := RandomNumberGenerator.new()
var board: PlaygroundBoard
var rack_bar: HBoxContainer
var preset_selector: OptionButton
var width_spin: SpinBox
var height_spin: SpinBox
var shape_selector: OptionButton
var adjacency_selector: OptionButton
var bonus_selector: OptionButton
var rack_size_spin: SpinBox
var apply_button: Button
var commit_button: Button
var clear_button: Button
var new_game_button: Button
var cpu_hint_button: Button
var cpu_play_button: Button
var moves_button: Button
var move_list: ItemList
var log_view: TextEdit
var score_label: Label
var preview_label: Label
var cpu_difficulty: OptionButton
var tile_lookup := {}
var layout := {}
var board_state := {}
var staged := {}
var highlight := {}
var last_move := {}
var selected_kind := ""
var rack_buttons: Array = []
func _ready() -> void:
  rng.randomize()
  eng = _instantiate_engine()
  if eng == null:
    push_error("WordEngine extension missing")
    return
  _build_ui()
  _populate_presets()
  _apply_preset_values("classic")
  _start_new_game()
func _build_ui() -> void:
  var root := VBoxContainer.new()
  root.anchor_right = 1.0
  root.anchor_bottom = 1.0
  root.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  root.size_flags_vertical = Control.SIZE_EXPAND_FILL
  add_child(root)
  var hero := VBoxContainer.new()
  hero.add_theme_constant_override("separation", 2)
  var title := Label.new()
  title.text = "TileTangle Playground"
  title.add_theme_font_size_override("font_size", 28)
  hero.add_child(title)
  var subtitle := Label.new()
  subtitle.text = "Design. Experiment. Solve."
  subtitle.add_theme_font_size_override("font_size", 16)
  hero.add_child(subtitle)
  root.add_child(hero)
  score_label = Label.new()
  score_label.text = ""
  root.add_child(score_label)
  var main := HBoxContainer.new()
  main.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  main.size_flags_vertical = Control.SIZE_EXPAND_FILL
  main.add_theme_constant_override("separation", 18)
  root.add_child(main)
  var left := VBoxContainer.new()
  left.custom_minimum_size = Vector2(250, 0)
  left.size_flags_vertical = Control.SIZE_EXPAND_FILL
  left.add_theme_constant_override("separation", 10)
  main.add_child(left)
  preset_selector = OptionButton.new()
  preset_selector.item_selected.connect(_on_preset_selected)
  left.add_child(_wrap_field("Quick presets", preset_selector))
  var dims := HBoxContainer.new()
  left.add_child(_wrap_field_container("Dimensions", dims))
  width_spin = SpinBox.new()
  width_spin.min_value = 3
  width_spin.max_value = 25
  width_spin.step = 1
  width_spin.custom_minimum_size = Vector2(80, 0)
  dims.add_child(width_spin)
  height_spin = SpinBox.new()
  height_spin.min_value = 3
  height_spin.max_value = 25
  height_spin.step = 1
  height_spin.custom_minimum_size = Vector2(80, 0)
  dims.add_child(height_spin)
  shape_selector = OptionButton.new()
  for shape in PlaygroundConfig.BOARD_SHAPES:
    shape_selector.add_item(shape.capitalize())
  left.add_child(_wrap_field("Board shape", shape_selector))
  adjacency_selector = OptionButton.new()
  adjacency_selector.add_item("Orthogonal")
  adjacency_selector.add_item("Diagonal")
  adjacency_selector.add_item("Hex")
  left.add_child(_wrap_field("Adjacency", adjacency_selector))
  bonus_selector = OptionButton.new()
  for preset in ["Auto", "Classic", "Hex", "Triangle", "Ring", "None"]:
    bonus_selector.add_item(preset)
  left.add_child(_wrap_field("Bonuses", bonus_selector))
  rack_size_spin = SpinBox.new()
  rack_size_spin.min_value = 3
  rack_size_spin.max_value = 12
  rack_size_spin.step = 1
  left.add_child(_wrap_field("Rack size", rack_size_spin))
  apply_button = Button.new()
  apply_button.text = "Apply changes"
  apply_button.pressed.connect(_start_new_game)
  left.add_child(apply_button)
  var mid := PanelContainer.new()
  mid.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  mid.size_flags_vertical = Control.SIZE_EXPAND_FILL
  mid.add_theme_constant_override("panel", 0)
  main.add_child(mid)
  var mid_content := VBoxContainer.new()
  mid_content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  mid_content.size_flags_vertical = Control.SIZE_EXPAND_FILL
  mid.add_child(mid_content)
  board = PlaygroundBoard.new()
  board.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  board.size_flags_vertical = Control.SIZE_EXPAND_FILL
  board.cell_pressed.connect(_on_board_cell)
  board.tile_dropped_on_cell.connect(_on_tile_dropped)
  mid_content.add_child(board)
  preview_label = Label.new()
  preview_label.text = ""
  mid_content.add_child(preview_label)
  var right := VBoxContainer.new()
  right.custom_minimum_size = Vector2(260, 0)
  right.size_flags_vertical = Control.SIZE_EXPAND_FILL
  right.add_theme_constant_override("separation", 8)
  main.add_child(right)
  cpu_difficulty = OptionButton.new()
  cpu_difficulty.add_item("Easy")
  cpu_difficulty.add_item("Medium")
  cpu_difficulty.add_item("Hard")
  right.add_child(_wrap_field("CPU difficulty", cpu_difficulty))
  cpu_hint_button = Button.new()
  cpu_hint_button.text = "CPU hint"
  cpu_hint_button.pressed.connect(_cpu_hint)
  right.add_child(cpu_hint_button)
  cpu_play_button = Button.new()
  cpu_play_button.text = "CPU move"
  cpu_play_button.pressed.connect(_cpu_play)
  right.add_child(cpu_play_button)
  moves_button = Button.new()
  moves_button.text = "Show candidates"
  moves_button.pressed.connect(_show_moves)
  right.add_child(moves_button)
  move_list = ItemList.new()
  move_list.custom_minimum_size = Vector2(0, 140)
  right.add_child(move_list)
  log_view = TextEdit.new()
  log_view.editable = false
  log_view.custom_minimum_size = Vector2(0, 140)
  right.add_child(log_view)
  var bottom_panel := PanelContainer.new()
  bottom_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
  bottom_panel.add_theme_constant_override("panel", 0)
  root.add_child(bottom_panel)
  var bottom := VBoxContainer.new()
  bottom.add_theme_constant_override("separation", 6)
  bottom_panel.add_child(bottom)
  rack_bar = HBoxContainer.new()
  rack_bar.add_theme_constant_override("separation", 8)
  bottom.add_child(rack_bar)
  var actions := HBoxContainer.new()
  actions.add_theme_constant_override("separation", 10)
  bottom.add_child(actions)
  commit_button = Button.new()
  commit_button.text = "Commit move"
  commit_button.pressed.connect(_commit_move)
  actions.add_child(commit_button)
  clear_button = Button.new()
  clear_button.text = "Clear"
  clear_button.pressed.connect(_clear_staged)
  actions.add_child(clear_button)
  new_game_button = Button.new()
  new_game_button.text = "Random rack"
  new_game_button.pressed.connect(_start_new_game)
  actions.add_child(new_game_button)
func _instantiate_engine():
  if EngineBridge:
    var e = EngineBridge.new_engine()
    if e != null:
      return e
  if ClassDB.class_exists("WordEngine"):
    return ClassDB.instantiate("WordEngine")
  return null
func _wrap_field(label_text: String, node: Control) -> VBoxContainer:
  var box := VBoxContainer.new()
  var lbl := Label.new()
  lbl.text = label_text
  box.add_child(lbl)
  box.add_child(node)
  return box
func _wrap_field_container(label_text: String, container: Control) -> VBoxContainer:
  var box := VBoxContainer.new()
  var lbl := Label.new()
  lbl.text = label_text
  box.add_child(lbl)
  box.add_child(container)
  return box
func _populate_presets() -> void:
  preset_selector.clear()
  var opts := PlaygroundConfig.preset_options()
  opts.sort_custom(func(a, b): return String(a.label).naturalnocasecmp_to(String(b.label)))
  for entry in opts:
    preset_selector.add_item(entry.label)
    preset_selector.set_item_metadata(preset_selector.item_count - 1, entry.value)
func _apply_preset_values(key: String) -> void:
  var preset := PlaygroundConfig.resolve_preset(key)
  width_spin.value = preset.width
  height_spin.value = preset.height
  rack_size_spin.value = preset.rack
  var shape_index := PlaygroundConfig.BOARD_SHAPES.find(preset.shape)
  if shape_index >= 0:
    shape_selector.select(shape_index)
  var adj_index := 0
  match preset.adjacency:
    "diagonal": adj_index = 1
    "hex": adj_index = 2
    _:
      adj_index = 0
  adjacency_selector.select(adj_index)
  match preset.bonus:
    "classic": bonus_selector.select(1)
    "hex": bonus_selector.select(2)
    "triangle": bonus_selector.select(3)
    "ring": bonus_selector.select(4)
    "none": bonus_selector.select(5)
    _:
      bonus_selector.select(0)
func _on_preset_selected(index: int) -> void:
  var meta: Variant = preset_selector.get_item_metadata(index)
  if meta is String:
    _apply_preset_values(meta)
func _start_new_game() -> void:
  var preset_index := preset_selector.get_selected()
  var preset_key: Variant = preset_selector.get_item_metadata(preset_index)
  if not (preset_key is String):
    preset_key = "classic"
  var adjacency_options := ["orthogonal", "diagonal", "hex"]
  var settings := {
  "preset": preset_key,
  "width": int(width_spin.value),
  "height": int(height_spin.value),
  "shape": PlaygroundConfig.BOARD_SHAPES[shape_selector.get_selected()],
  "adjacency": adjacency_options[adjacency_selector.get_selected()],
  "bonus": _bonus_key(),
  "rack_size": int(rack_size_spin.value),
  "rng_seed": _next_seed(),
  "free_word_mode": true,
  }
  var result := PlaygroundConfig.build_config(settings)
  var config_json := JSON.stringify(result.config)
  if not eng.new_game(config_json, 2):
    push_error("Failed to initialize game")
    return
  if result.bonuses.size() > 0:
    var ok: bool = eng.set_bonuses(JSON.stringify(result.bonuses))
    if not ok:
      push_warning("Failed to apply bonuses")
  tile_lookup = {}
  for kind in result.tileset.tile_kinds:
    tile_lookup[kind.id] = {
      "symbol": kind.get("symbol", kind.id),
      "score": kind.get("score", 0),
      "is_blank": kind.get("is_blank", false),
    }
  layout = result.layout
  staged.clear()
  highlight.clear()
  last_move.clear()
  board.configure(layout, tile_lookup, result.bonuses)
  _refresh_all()
func _bonus_key() -> String:
  match bonus_selector.get_selected():
    1:
      return "classic"
    2:
      return "hex"
    3:
      return "triangle"
    4:
      return "ring"
    5:
      return "none"
  return "auto"
func _next_seed() -> int:
  var hi := int(rng.randi())
  var lo := int(rng.randi())
  var seed := ((hi & 0xffffffff) << 32) ^ (lo & 0xffffffff)
  return abs(seed) + 1
func _refresh_all() -> void:
  _refresh_board_state()
  _refresh_rack()
  _update_scores()
  _update_preview_label("")
  _refresh_event_log()
func _refresh_board_state() -> void:
  var json: String = eng.get_board_cells_json()
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    return
  board_state.clear()
  for cell in parsed.get("cells", []):
    var key := "%d,%d" % [int(cell.x), int(cell.y)]
    var top: Variant = cell.get("top", null)
    if top != null:
      var kind_id := String(top.get("kind_id", ""))
      var lookup: Dictionary = tile_lookup.get(kind_id, {})
      var symbol: String = String(lookup.get("symbol", kind_id))
      if lookup.get("is_blank", false) and top.has("mark") and top.mark != null:
        symbol = String(top.mark)
      var score: int = int(lookup.get("score", 0))
      board_state[key] = {
      "kind_id": kind_id,
      "symbol": symbol,
      "score": score,
      }
  board.update_board(board_state, _staged_tiles(), _compose_highlights())
func _staged_tiles() -> Dictionary:
  var out := {}
  for key in staged.keys():
    out[key] = staged[key]
  return out
func _compose_highlights() -> Dictionary:
  var map := {}
  for k in last_move.keys():
    map[k] = last_move[k]
  for k in highlight.keys():
    map[k] = highlight[k]
  return map
func _refresh_rack() -> void:
  if rack_bar == null:
    return
  for child in rack_bar.get_children():
    child.queue_free()
  rack_buttons.clear()
  var json: String = eng.get_rack_json()
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    return
  for entry in parsed:
    var btn := PlaygroundRackTile.new()
    var symbol: String = String(entry.get("symbol", entry.get("kind_id", "")))
    var info: Dictionary = {
      "kind_id": entry.get("kind_id", ""),
      "symbol": symbol,
      "score": entry.get("score", 0),
    }
    btn.set_tile(info)
    btn.pressed.connect(func(): _select_rack_tile(btn))
    rack_bar.add_child(btn)
    rack_buttons.append(btn)
  if rack_buttons.size() > 0:
    _select_rack_tile(rack_buttons[0])
func _select_rack_tile(btn: PlaygroundRackTile) -> void:
  selected_kind = btn.kind_id
  for other in rack_buttons:
    other.select(other == btn)
func _update_scores() -> void:
  var json: String = eng.get_scores_json()
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    score_label.text = ""
    return
  var players: Array = parsed.get("players", [])
  var to_move := int(parsed.get("to_move", 0))
  var you := 0
  var cpu := 0
  if players.size() > 0:
    you = int(players[0].get("score", 0))
  if players.size() > 1:
    cpu = int(players[1].get("score", 0))
  var turn := "Your turn" if to_move == 0 else "CPU thinking"
  score_label.text = "You: %d | CPU: %d – %s" % [you, cpu, turn]
func _update_preview_label(text: String) -> void:
  preview_label.text = text
func _refresh_event_log() -> void:
  var json: String = eng.get_event_log_json(MAX_EVENT_LOG)
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    return
  var lines := []
  for entry in parsed:
    var turn := int(entry.get("turn", 0))
    var player := int(entry.get("player", 0))
    var who := "You" if player == 0 else "CPU"
    match entry.get("type", ""):
      "play":
        lines.append("Turn %d: %s played %d" % [turn, who, int(entry.get("total", entry.get("score", 0)))])
      "draw":
        lines.append("Turn %d: %s drew %s" % [turn, who, String(entry.get("tiles", []) )])
      "exchange":
        lines.append("Turn %d: %s exchanged" % [turn, who])
      "pass":
        lines.append("Turn %d: %s passed" % [turn, who])
      _:
        lines.append("Turn %d: %s" % [turn, who])
  log_view.text = "\n".join(lines)
func _place_tile(key: String, kind_id: String) -> void:
  if not tile_lookup.has(kind_id):
    return
  var info: Dictionary = tile_lookup[kind_id]
  staged[key] = {
  "kind_id": kind_id,
  "symbol": info.get("symbol", kind_id),
  "score": info.get("score", 0),
  }
  highlight.clear()
  _queue_preview()
func _remove_staged(key: String) -> void:
  if staged.has(key):
    staged.erase(key)
    highlight.clear()
    board.update_board(board_state, _staged_tiles(), _compose_highlights())
    _update_preview_label("")
func _on_board_cell(x: int, y: int) -> void:
  var key := "%d,%d" % [x, y]
  if staged.has(key):
    _remove_staged(key)
    return
  if selected_kind == "":
    return
  if board_state.has(key):
    push_warning("Cell already occupied")
    return
  _place_tile(key, selected_kind)
func _on_tile_dropped(x: int, y: int, kind_id: String) -> void:
  var key := "%d,%d" % [x, y]
  if board_state.has(key):
    return
  _place_tile(key, kind_id)
func _queue_preview() -> void:
  if staged.is_empty():
    highlight.clear()
    board.update_board(board_state, _staged_tiles(), _compose_highlights())
    _update_preview_label("")
    return
  var placements := []
  for key in staged.keys():
    var parts: Array = key.split(",")
    placements.append({
      "x": int(parts[0]),
      "y": int(parts[1]),
      "kind_id": String(staged[key].get("kind_id", "")),
    })
  var json: String = eng.preview_move(JSON.stringify(placements))
  if json == "":
    highlight["invalid"] = true
    board.update_board(board_state, _staged_tiles(), _compose_highlights())
    _update_preview_label("Preview: invalid move")
    return
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    return
  highlight.clear()
  var main_cells: Array = parsed.get("main_cells", [])
  var cross_cells: Array = parsed.get("cross_cells", [])
  highlight["preview_main"] = main_cells
  highlight["preview_cross"] = cross_cells
  var total := int(parsed.get("total", 0))
  var main_word := String(parsed.get("main_word", ""))
  _update_preview_label("Preview: %s (%d)" % [main_word, total])
  board.update_board(board_state, _staged_tiles(), _compose_highlights())
func _commit_move() -> void:
  if staged.is_empty():
    return
  var placements := []
  for key in staged.keys():
    var parts: Array = key.split(",")
    placements.append({
      "x": int(parts[0]),
      "y": int(parts[1]),
      "kind_id": String(staged[key].get("kind_id", "")),
    })
  var preview_json: String = eng.preview_move(JSON.stringify(placements))
  var preview: Variant = JSON.parse_string(preview_json)
  var result: String = eng.play_move(JSON.stringify(placements))
  if result == "":
    push_error("Move rejected")
    return
  if preview != null:
    last_move.clear()
    last_move["last_main"] = preview.get("main_cells", [])
    last_move["last_cross"] = preview.get("cross_cells", [])
  staged.clear()
  highlight.clear()
  _refresh_all()
func _clear_staged() -> void:
  staged.clear()
  highlight.clear()
  board.update_board(board_state, _staged_tiles(), _compose_highlights())
  _update_preview_label("")
func _cpu_hint() -> void:
  var diff: String = ["easy", "medium", "hard"][cpu_difficulty.get_selected()]
  var json: String = eng.best_move(diff, _next_seed())
  if json == "" or json == "null":
    push_warning("No hint available")
    return
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    return
  highlight.clear()
  var preview_cells := []
  for entry in parsed.get("placements", []):
    preview_cells.append([int(entry.get("x", 0)), int(entry.get("y", 0))])
  highlight["preview_main"] = preview_cells
  _update_preview_label("CPU suggests %s (%d)" % [parsed.get("word", ""), int(parsed.get("total", parsed.get("score", 0)))])
  board.update_board(board_state, _staged_tiles(), _compose_highlights())
func _cpu_play() -> void:
  var diff: String = ["easy", "medium", "hard"][cpu_difficulty.get_selected()]
  var json: String = eng.best_move(diff, _next_seed())
  if json == "" or json == "null":
    push_warning("CPU has no moves")
    return
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    return
  var placements: Array = parsed.get("placements", [])
  if placements.is_empty():
    return
  var result: String = eng.play_move(JSON.stringify(placements))
  if result == "":
    push_error("CPU move failed")
    return
  last_move.clear()
  var last_cells := []
  for entry in placements:
    last_cells.append([int(entry.get("x", 0)), int(entry.get("y", 0))])
  last_move["last_main"] = last_cells
  staged.clear()
  highlight.clear()
  _refresh_all()
func _show_moves() -> void:
  var max_len := int(rack_size_spin.value)
  var json: String = eng.generate_moves(max_len, 12)
  var parsed: Variant = JSON.parse_string(json)
  if parsed == null:
    return
  move_list.clear()
  for entry in parsed:
    var word: String = String(entry.get("word", ""))
    var score: int = int(entry.get("score", 0))
    move_list.add_item("%s (%d)" % [word, score])
