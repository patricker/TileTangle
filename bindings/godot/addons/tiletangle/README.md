# TileTangle Godot Addon

This addon packages the TileTangle GDExtension library and helper scripts so you can drop the engine
into an existing Godot project. The packaged archives produced via `tools/package_release.py`
include prebuilt libraries for the host platform under `addons/tiletangle/bin/`.

To load the extension inside Godot:

1. Copy the `addons/tiletangle` folder into your project.
2. Ensure the `.gdextension` file is referenced in your project settings or autoloads if needed.
3. Use the provided scripts or create new nodes that call into the `TileTangle` API exposed by the
   extension.

See `bindings/godot/examples` for a minimal project that exercises the extension.
