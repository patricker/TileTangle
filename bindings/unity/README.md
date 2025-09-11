# Unity Bindings (C#)

This folder contains a thin C# wrapper around the C ABI provided by `engine_ffi` (`tiletangle_ffi`), suitable for use as a native plugin in Unity.

## Layout

- `Runtime/TileTangle.cs` — P/Invoke declarations and a small OO wrapper.
- `Examples/ConsoleSmoke.cs` — Minimal non-Unity console example using the wrapper.

## Building the native library

First build the shared library for your platform:

- Linux/macOS: `cargo build -p tiletangle-engine-ffi --release`
- Windows (MSVC): `cargo build -p tiletangle-engine-ffi --release --target x86_64-pc-windows-msvc`

The resulting artifacts are named (by platform):

- Linux: `target/release/libtiletangle_ffi.so`
- macOS: `target/release/libtiletangle_ffi.dylib`
- Windows: `target/release/tiletangle_ffi.dll`

Copy the file into your Unity project under `Assets/Plugins/<Platform>/` with the expected filename. Unity will load it via `DllImport("tiletangle_ffi")`.

## Using in Unity

1. Add `Assets/Plugins/<Platform>/libtiletangle_ffi.(so|dylib|dll)`.
2. Add `TileTangle.cs` to a suitable folder under your `Assets/` (e.g., `Assets/Scripts/TileTangle/`).
3. Call the wrapper:

```csharp
var engine = new TileTangle.Engine();
engine.NewGame(configJson, players: 2);
var scoreJson = engine.PlayMove("[{\"x\":7,\"y\":7,\"kind_id\":\"A\"}]");
var boardJson = engine.GetBoardJson();
engine.Dispose();
```

See `Examples/ConsoleSmoke.cs` for a runnable snippet (outside Unity) to sanity-check the native calls.

## Notes

- Strings are marshaled as UTF-8 null-terminated C strings; free any returned string via `tt_string_free` (handled by the wrapper).
- Retrieve last error message via `TileTangle.Engine.LastError()` if a call returns `null`.
- The engine expects a JSON config matching `Appendix A — Sample Configs` in the repo `TODO.md`.

