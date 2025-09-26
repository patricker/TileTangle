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

## Run and export from the CLI

- Headless run (useful for logs while iterating):

```
tools/godot-4.5/godot4 --headless --path bindings/godot/examples --verbose
```

- Web export (Godot preset “Web”):

```
make web-export
```

`make web-wire` copies the WASM package and injects a small shim into the exported `index.html` so the Playground scene can call the engine in the browser. It also copies dictionary assets into `dist/web/dictionaries/`; the shim auto-loads the dictionary so CPU and candidate generation are useful.

## Install the Godot CLI locally

The repo does not check in Godot binaries. To enable headless runs and exports:

1. Download the editor for your OS from the Godot 4.5 release.
   - Linux example: https://github.com/godotengine/godot/releases/download/4.5-stable/Godot_v4.5-stable_linux.x86_64.zip
2. Place the binary under `tools/godot-4.5/` and create a symlink named `godot4` to it.
3. Ensure it is executable (`chmod +x`).
4. Install export templates for 4.5 via the editor or into your platform’s templates directory.

After this, `make web-export` and `make docs-godot` find the CLI at `tools/godot-4.5/godot4`.

## Project setup

1. Use the example: `bindings/godot/examples/` (contains `project.godot`).
2. Copy the built library to `examples/godot/bin/<platform>/`.
3. Edit `examples/godot/tiletangle_godot.gdextension` to point to your library.
4. Open the project and run `scenes/Main.tscn`.
5. The extension registers a `WordEngine` class.

## One‑shot web publish (docs site)

To build the WASM package, export the Godot web build, wire the JS shim, and publish the result to the docs static folder in one step:

```
make docs-godot
```

This target copies the export into `docs/static/playground/`, and the page “Godot Web Export” embeds it via an iframe.

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

### Web vs. native

- Native builds use the GDExtension class `WordEngine` directly.
- Web builds route engine calls through `EngineBridge.gd` to `window.TT`, which is provided by `tools/web/web_engine.js` and backed by the WASM engine.
