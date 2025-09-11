# Godot Bindings (GDExtension)

This crate exposes a `WordEngine` Godot class backed by the Rust core engine. It provides:

- `new_game(config_json: String, players: int) -> bool`
- `play_move(placements_json: String) -> String` (score JSON, empty on error)
- `get_board_json() -> String` (board JSON)
- `set_free_word_mode(on: bool)`

## Build

```
cargo build -p tiletangle-godot --release
```

The built library will be at:

- Linux: `target/release/libtiletangle_godot.so`
- macOS: `target/release/libtiletangle_godot.dylib`
- Windows: `target/release/tiletangle_godot.dll`

## Use in Godot 4

1. Create a new Godot project.
2. Place the built library under `res://bin/<platform>/` and create a `*.gdextension` file referencing it.
3. Ensure the entry symbol is set by godot-rust macro (`godot_init`).
4. Add a script to create and use the class:

```gdscript
var eng := WordEngine.new()
var ok := eng.new_game(config_json, 2)
if ok:
    var score_json := eng.play_move('[{"x":7,"y":7,"kind_id":"A"}]')
    var board_json := eng.get_board_json()
```

Refer to godot-rust documentation for a template `.gdextension` and platform-specific library names.

