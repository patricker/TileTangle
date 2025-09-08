use engine::{self, BoardGeometry, Rules};
use serde::Deserialize;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = RefCell::new(None);
}

fn set_error(msg: impl ToString) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(msg.to_string()));
}

fn take_cstring(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_else(|_| CString::new("invalid utf8").unwrap()).into_raw()
}

#[repr(C)]
pub struct GameHandle {
    _private: [u8; 0],
}

struct FfiGame {
    state: engine::GameState,
    rules: engine::CrosswordRules,
}

#[derive(Deserialize)]
struct JsTileKind {
    id: String,
    symbol: String,
    score: i16,
    #[serde(default)]
    is_blank: bool,
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Deserialize)]
struct JsTileset {
    tile_kinds: Vec<JsTileKind>,
}

#[derive(Deserialize)]
struct JsRectBoardLayout {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct JsConfig {
    tileset: JsTileset,
    rack_size: usize,
    board_layout: JsRectBoardLayout,
    ruleset_id: String,
    dictionary_id: String,
    rng_seed: u64,
    #[serde(default)]
    tile_counts: std::collections::HashMap<String, u32>,
    #[serde(default)]
    free_word_mode: bool,
}

#[derive(Deserialize)]
struct JsPlacement {
    x: i32,
    y: i32,
    kind_id: String,
}

/// Returns null on error; call `tt_last_error_message()` to retrieve the error string.
#[no_mangle]
pub extern "C" fn tt_new_game(config_json: *const c_char, players: c_uint) -> *mut GameHandle {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    if config_json.is_null() {
        set_error("config_json is null");
        return std::ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(config_json) };
    let cfg_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("config_json is not valid UTF-8");
            return std::ptr::null_mut();
        }
    };
    let cfg: JsConfig = match serde_json::from_str(cfg_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("config parse error: {}", e));
            return std::ptr::null_mut();
        }
    };
    let tileset = engine::Tileset {
        tile_kinds: cfg
            .tileset
            .tile_kinds
            .into_iter()
            .map(|k| engine::TileKind {
                id: k.id,
                symbol: k.symbol,
                score: k.score,
                is_blank: k.is_blank,
                aliases: k.aliases,
            })
            .collect(),
    };
    let eng_cfg = engine::GameConfig {
        tileset,
        rack_size: cfg.rack_size,
        board_layout: engine::RectBoardLayout {
            width: cfg.board_layout.width,
            height: cfg.board_layout.height,
        },
        ruleset_id: cfg.ruleset_id,
        dictionary_id: cfg.dictionary_id,
        rng_seed: cfg.rng_seed,
        tile_counts: cfg.tile_counts,
    };
    let state = match engine::GameState::new(&eng_cfg, players as usize) {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("{}", e));
            return std::ptr::null_mut();
        }
    };
    let rules = engine::CrosswordRules {
        free_word_mode: cfg.free_word_mode,
        ..Default::default()
    };
    let boxed = Box::new(FfiGame { state, rules });
    Box::into_raw(boxed) as *mut GameHandle
}

#[no_mangle]
pub extern "C" fn tt_free_game(game: *mut GameHandle) {
    if game.is_null() {
        return;
    }
    unsafe { drop(Box::from_raw(game as *mut FfiGame)) };
}

/// On success, returns a newly-allocated C string containing a JSON score object.
/// On error, returns null; use `tt_last_error_message()` for details.
#[no_mangle]
pub extern "C" fn tt_play_move(game: *mut GameHandle, placements_json: *const c_char) -> *mut c_char {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    if game.is_null() {
        set_error("game is null");
        return std::ptr::null_mut();
    }
    if placements_json.is_null() {
        set_error("placements_json is null");
        return std::ptr::null_mut();
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    let cstr = unsafe { CStr::from_ptr(placements_json) };
    let p_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("placements_json is not valid UTF-8");
            return std::ptr::null_mut();
        }
    };
    let placements: Vec<JsPlacement> = match serde_json::from_str(p_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("placements parse error: {}", e));
            return std::ptr::null_mut();
        }
    };
    let mut mv = engine::MoveDraft { placements: vec![] };
    for p in placements {
        let cid = match g
            .state
            .board
            .geom
            .to_cell_id(engine::Coord2D { x: p.x, y: p.y })
        {
            Some(id) => id,
            None => {
                set_error("invalid coordinates");
                return std::ptr::null_mut();
            }
        };
        mv.placements.push((
            cid,
            engine::Tile {
                kind_id: p.kind_id,
                mark: None,
            },
        ));
    }
    let validated = match g.rules.validate(&g.state, &mv) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("{}", e));
            return std::ptr::null_mut();
        }
    };
    let score = g.rules.score(&g.state, &validated);
    if score.main_score < 0 {
        set_error("invalid word(s)");
        return std::ptr::null_mut();
    }
    if let Err(e) = g.rules.commit(&mut g.state, validated, &score) {
        set_error(format!("{}", e));
        return std::ptr::null_mut();
    }
    let val = serde_json::json!({
        "total": score.total,
        "main_word": score.main_word,
        "main_score": score.main_score,
        "cross_words": score.cross_words,
        "bingo": score.bingo,
    });
    take_cstring(serde_json::to_string(&val).unwrap())
}

/// Returns a newly-allocated C string with board JSON: { width, height, rows }.
#[no_mangle]
pub extern "C" fn tt_get_board(game: *const GameHandle) -> *mut c_char {
    if game.is_null() {
        return std::ptr::null_mut();
    }
    let g = unsafe { &*(game as *const FfiGame) };
    let w = g.state.board.geom.width as i32;
    let h = g.state.board.geom.height as i32;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for y in 0..h {
        let mut row = Vec::new();
        for x in 0..w {
            let id = g
                .state
                .board
                .geom
                .to_cell_id(engine::Coord2D { x, y })
                .unwrap();
            let cell = &g.state.board.cells[id.0 as usize];
            let s = if let Some(t) = cell.stack.last() {
                t.kind_id.clone()
            } else {
                String::from("")
            };
            row.push(s);
        }
        rows.push(row);
    }
    let json = serde_json::json!({"width": w, "height": h, "rows": rows});
    take_cstring(serde_json::to_string(&json).unwrap())
}

/// Free a C string previously returned by this library.
#[no_mangle]
pub extern "C" fn tt_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Return the last error message for the current thread (pointer is valid until next call).
#[no_mangle]
pub extern "C" fn tt_last_error_message() -> *const c_char {
    LAST_ERROR.with(|e| {
        if let Some(ref s) = *e.borrow() {
            // store string in thread local again to keep owned CString memory
            // We avoid allocating per call by caching CString bytes in another TLS if needed.
            // Simpler: allocate a new CString each call; caller copies string immediately.
            let cs = CString::new(s.as_str()).unwrap_or_else(|_| CString::new("error").unwrap());
            let ptr = cs.as_ptr();
            // Leak the CString; caller should not free this (valid until next process end). For simplicity.
            std::mem::forget(cs);
            ptr
        } else {
            std::ptr::null()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_json() -> String {
        serde_json::json!({
            "tileset": {"tile_kinds": [
                {"id": "A", "symbol": "A", "score": 1},
                {"id": "B", "symbol": "B", "score": 3}
            ]},
            "rack_size": 7,
            "board_layout": {"width": 5, "height": 5},
            "ruleset_id": "cross",
            "dictionary_id": "en",
            "rng_seed": 42,
            "tile_counts": {"A": 10, "B": 10},
            "free_word_mode": true
        }).to_string()
    }

    #[test]
    fn smoke_create_get_board_and_play() {
        let cfg = CString::new(cfg_json()).unwrap();
        let game = tt_new_game(cfg.as_ptr(), 2);
        assert!(!game.is_null(), "should create game: {:?}", unsafe {
            CStr::from_ptr(tt_last_error_message()).to_string_lossy().into_owned()
        });
        let b = tt_get_board(game);
        assert!(!b.is_null());
        unsafe { tt_string_free(b) };
        // place A at center then B to the right
        let placements = serde_json::json!([
            {"x": 2, "y": 2, "kind_id": "A"},
            {"x": 3, "y": 2, "kind_id": "B"}
        ])
        .to_string();
        let p = CString::new(placements).unwrap();
        let res = tt_play_move(game, p.as_ptr());
        assert!(!res.is_null(), "play_move error: {:?}", unsafe {
            let e = tt_last_error_message();
            if e.is_null() { String::from("(none)") } else { CStr::from_ptr(e).to_string_lossy().into_owned() }
        });
        unsafe { tt_string_free(res) };
        tt_free_game(game);
    }
}

