---
sidebar_position: 9
---

# Unity Integration

This guide shows how to call the Rust engine from Unity via a native plugin (`tiletangle_ffi`) and a small C# wrapper.

## Build the native library

- Linux/macOS: `make unity-libs`
- Windows: `cargo build -p tiletangle-engine-ffi --release --target x86_64-pc-windows-msvc`

Artifacts:

- Linux: `target/release/libtiletangle_ffi.so`
- macOS: `target/release/libtiletangle_ffi.dylib`
- Windows: `target/release/tiletangle_ffi.dll`

Copy to your Unity project: `Assets/Plugins/<Platform>/`.

## Single download bundle (CI)

- CI produces a single archive containing:
  - `com.tiletangle.engine/` — the Unity UPM package with the C# wrapper and samples.
  - `Plugins/` — prebuilt native libraries for Windows, macOS, and Linux, plus C header.
- Download artifact: `unity-bundle` from the CI run, then:
  1. Extract the archive anywhere.
  2. Copy the appropriate native library from `Plugins/<Platform>/` into `Assets/Plugins/<Platform>/` in your Unity project.
  3. In Unity → Package Manager → Add package from disk… select the extracted `com.tiletangle.engine` folder.
  4. Open the sample scene or add `BoardDemo` to an empty scene and press Play.

## C# wrapper

Use `bindings/unity/Runtime/TileTangle.cs` directly in your project. It exposes:

- `Engine.NewGame(configJson, players)`
- `Engine.PlayMove(placementsJson) -> string?`
- `Engine.GetBoardJson() -> string?`
- `Engine.LastError() -> string?`

### Example

```csharp
using TileTangle;

var engine = new Engine();
engine.NewGame(configJson, 2);
var scoreJson = engine.PlayMove("[{\"x\":7,\"y\":7,\"kind_id\":\"A\"}]");
var boardJson = engine.GetBoardJson();
engine.Dispose();
```

## Minimal UI

- Create a Canvas with a `GridLayoutGroup`.
- Add a `Button` prefab child with a `TMP_Text` (optional).
- Add `bindings/unity/Runtime/UGUISample.cs` to a GameObject and assign:
  - `gridRoot`: the RectTransform that has the `GridLayoutGroup`.
  - `cellButtonPrefab`: your Button prefab.
- Optional: add a Toggle and hook it to `UGUISample.OnToggleFreeWordMode(bool)` to switch rules mid-session.
- Enter Play Mode and click cells to place the configured tile (default `A`).

Example code paths:
- Wrapper: `bindings/unity/Runtime/TileTangle.cs`
- UGUI sample: `bindings/unity/Runtime/UGUISample.cs`
- Playmode test: `bindings/unity/Tests/PlayMode/EngineSmokeTest.cs`

## Troubleshooting

- If `DllNotFoundException`: ensure the plugin filename and path match the platform.
- If `PlayMove` returns null: check `Engine.LastError()` for a message.
