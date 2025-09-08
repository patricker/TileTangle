---
title: C ABI
---

This crate exposes a small, stable C API for embedding.

Header generation:

- `make ffi-header` (generates `engine_ffi/include/engine.h`)

API (excerpt):

- `tt_new_game(const char* config_json, unsigned int players) -> GameHandle*`
- `tt_free_game(GameHandle* game)`
- `tt_play_move(GameHandle* game, const char* placements_json) -> char*` (JSON score)
- `tt_get_board(const GameHandle* game) -> char*` (JSON board)
- `tt_string_free(char* s)`
- `tt_last_error_message() -> const char*`

Lifecycle:

- Create with `tt_new_game`.
- Query board via `tt_get_board`.
- Play moves with `tt_play_move`.
- Free returned strings with `tt_string_free`.
- Destroy with `tt_free_game`.

Error handling:

- On failure, functions return `NULL`. Call `tt_last_error_message()` and copy the string immediately.

