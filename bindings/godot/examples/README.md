# Godot Example

This Godot 4 sample now ships with two surfaces:

* **Playground.tscn** – a full-featured control room inspired by the web playground. Mix board presets, adjacency modes, bonus layouts, and automation without reloading the engine.
* **Main.tscn / Main3D.tscn** – the original lightweight smoke scenes for quick validation.

## Steps

1. Build the extension: `make godot-build`.
2. Copy the built library to `examples/godot/bin/<platform>/` and edit `examples/godot/tiletangle_godot.gdextension` to point to it.
3. Open this folder as a Godot project and run `scenes/Playground.tscn`.

The playground mirrors the Docusaurus experience: choose a preset, tweak shape/adjacency/bonuses, and hit **Apply changes**. Tiles can be dragged from the rack or single-clicked onto the board. Previews highlight main/cross words, CPU hints surface best moves, and the right rail lists generated candidates plus the running event log.

If you need a minimal sandbox or stacked-board demo, `scenes/Main.tscn` and `scenes/Main3D.tscn` remain available. They render simple buttons over a 2D grid: click to place the selected rack tile, use “Commit Move” to apply staged placements, and toggle “Hex Geometry” for a quick graph adjacency check.

Open `scenes/Main3D.tscn` to explore stacked boards. A Canvas overlay lets you scrub through vertical slices and trigger a pre-baked “Play sample vertical word” action that drops tiles across layers, demonstrating how 3D connectivity works.

## Assets

Place any custom textures (logos, tile art, favicons) under `res://assets/` (see `assets/README.md`). Godot imports everything in that folder automatically, so once the files land there you can select them from the resource picker without touching an additional build step.
