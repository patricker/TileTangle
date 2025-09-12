# Godot Example

This minimal Godot 4 example uses the `WordEngine` class from the Rust GDExtension to create a board and place tiles. It includes UI toggles for Free Word Mode and Hex geometry.

## Steps

1. Build the extension: `make godot-build`.
2. Copy the built library to `examples/godot/bin/<platform>/` and edit `examples/godot/tiletangle_godot.gdextension` to point to it.
3. Open this folder as a Godot project and run `scenes/Main.tscn`.

The UI renders a simple grid of buttons; clicking a cell places an `A` tile. Toggle “Hex Geometry” to switch to a graph-based hex adjacency.
