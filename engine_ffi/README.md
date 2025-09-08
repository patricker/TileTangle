# Engine FFI (C ABI)

C ABI wrapper around the core `tiletangle-engine` crate.

Build the shared library and generate the C header:

- cargo build -p tiletangle-engine-ffi --release
- cbindgen --config cbindgen.toml --crate tiletangle-engine-ffi --output include/engine.h

API

- `tt_new_game(config_json: *const c_char, players: c_uint) -> *mut GameHandle`
- `tt_free_game(game: *mut GameHandle)`
- `tt_play_move(game: *mut GameHandle, placements_json: *const c_char) -> *mut c_char` (JSON score)
- `tt_get_board(game: *const GameHandle) -> *mut c_char` (JSON board)
- `tt_string_free(ptr: *mut c_char)`
- `tt_last_error_message() -> *const c_char`

Notes

- Returned strings must be freed via `tt_string_free`.
- Error strings should be copied immediately by the caller (pointer may be invalidated by subsequent calls).
