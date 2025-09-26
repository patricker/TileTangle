extends Node

class_name EngineBridge

# Thin factory returning an object that implements the engine API used by the playground.
# On native platforms, uses the GDExtension class `WordEngine`.
# On Web, returns a stub that can be wired to a WASM/JS module later.

static func new_engine():
  if OS.has_feature("web"):
    return _WebWasmEngine.new()
  if ClassDB.class_exists("WordEngine"):
    return ClassDB.instantiate("WordEngine")
  return null

class _WebWasmEngine:
  static func _call(name: String, args: Array = []):
    var code := "(function(){ try { if (window.TT && TT." + name + ") { return TT." + name + ".apply(TT, ARGS); } } catch(e){ console.error(e); } return null; })()"
    code = code.replace("ARGS", JSON.stringify(args))
    return JavaScriptBridge.eval(code)

  func new_game(_config_json: String, _players: int) -> bool:
    _call("init", [])
    var ok = _call("new_game", [_config_json, _players])
    return bool(ok)

  func set_bonuses(_bonuses_json: String) -> bool:
    var ok = _call("set_bonuses", [_bonuses_json])
    return bool(ok)

  func get_board_cells_json() -> String:
    var s = _call("get_board_cells_json", [])
    return String(s) if s != null else "{}"

  func get_rack_json() -> String:
    var s = _call("get_rack_json", [])
    return String(s) if s != null else "[]"

  func get_scores_json() -> String:
    var s = _call("get_scores_json", [])
    return String(s) if s != null else "{}"

  func get_event_log_json(_limit: int) -> String:
    var s = _call("get_event_log_json", [_limit])
    return String(s) if s != null else "[]"

  func preview_move(_placements_json: String) -> String:
    var s = _call("preview_move", [_placements_json])
    return String(s) if s != null else ""

  func play_move(_placements_json: String) -> String:
    var s = _call("play_move", [_placements_json])
    return String(s) if s != null else ""

  func best_move(_difficulty: String, _seed: int) -> String:
    var s = _call("best_move", [_difficulty, _seed])
    return String(s) if s != null else ""

  func generate_moves(_max_len: int, _limit: int) -> String:
    var s = _call("generate_moves", [_max_len, _limit])
    return String(s) if s != null else "[]"
