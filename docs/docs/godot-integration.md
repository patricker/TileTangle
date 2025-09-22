---
sidebar_position: 10
---

# Godot Integration

This guide integrates the Rust engine with Godot 4 via GDExtension using `godot-rust`.

## Build the extension

```
make godot-build
```

Artifacts:

- Linux: `target/release/libtiletangle_godot.so`
- macOS: `target/release/libtiletangle_godot.dylib`
- Windows: `target/release/tiletangle_godot.dll`

## Project setup

1. Use the example: `bindings/godot/examples/` (contains `project.godot`).
2. Copy the built library to `examples/godot/bin/<platform>/`.
3. Edit `examples/godot/tiletangle_godot.gdextension` to point to your library.
4. Open the project and run `scenes/Main.tscn`.
5. The extension registers a `WordEngine` class.

## 3D Scene (layers)

- Open `scenes/Main3D.tscn` from the example project.
- Script `scripts/Board3D.gd` initializes a 3D layout (`type: "3d"`) and renders stacked layers.
- Symbols are drawn as `Label3D` nodes above each cell. Adjust `W/H/D` in the script for different sizes.

## API

- `WordEngine.new_game(config_json: String, players: int) -> bool`
- `WordEngine.play_move(placements_json: String) -> String` (empty on error)
- `WordEngine.get_board_json() -> String`
- `WordEngine.set_free_word_mode(on: bool)`

## Toggle rules mid-session

- In the 2D example (`scripts/Board.gd`), a top bar button toggles free-word mode by calling `eng.set_free_word_mode(...)`.

### GDScript example

```gdscript
var eng := WordEngine.new()
var ok := eng.new_game(CONFIG_JSON, 2)
if not ok:
  push_error("Failed to init engine")
  return
var score_json := eng.play_move('[{"x":7,"y":7,"kind_id":"A"}]')
print("Score:", score_json)
print("Board:", eng.get_board_json())
```

## Notes

- The engine uses a JSON configuration. See the Classic demo and the Playground’s Snapshot tab for working
  examples you can copy, or check `bindings/godot/examples/` for sample configs used by the integration.
- Example project paths:
  - `bindings/godot/examples/scenes/Main.tscn`
  - `bindings/godot/examples/scripts/Board.gd`
  - `bindings/godot/examples/scripts/TestSmoke.gd`
