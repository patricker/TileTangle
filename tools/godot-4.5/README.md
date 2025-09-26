Local Godot CLI (not checked in)

This folder is ignored by git. Place a Godot 4.5 editor/CLI here for headless builds and exports used by the examples and Makefile targets.

Linux

- Download: https://github.com/godotengine/godot/releases/download/4.5-stable/Godot_v4.5-stable_linux.x86_64.zip
- Unzip into this folder so the binary is `tools/godot-4.5/Godot_v4.5-stable_linux.x86_64`.
- Create a symlink named `godot4` pointing to the binary:

```
cd tools/godot-4.5
ln -s Godot_v4.5-stable_linux.x86_64 godot4
chmod +x Godot_v4.5-stable_linux.x86_64
```

macOS

- Download: https://github.com/godotengine/godot/releases/tag/4.5-stable (macOS editor build)
- Place the `.app` or the `godot` CLI in this folder and create `godot4` as a symlink to the CLI binary.

Windows

- Download the Windows editor from the same release page.
- Place the `Godot_v4.5-stable_win64.exe` here and create `godot4.bat` that calls it or adjust the Makefile to reference your path.

Export templates

- Install the Godot 4.5 export templates through the editor or manually into `~/.local/share/godot/export_templates/4.5.stable` (Linux). Other OS paths follow Godot’s defaults.

After this setup, the following just work:

- `make web-export` — exports the Godot example to Web.
- `make docs-godot` — builds WASM, exports Web, wires the shim, and copies into `docs/static/playground/` for docs embedding.

