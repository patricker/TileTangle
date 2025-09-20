use engine::{self, AiConfig, AiDifficulty, BoardGeometry, Rules};
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};
use std::slice;

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn set_error(msg: impl ToString) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(msg.to_string()));
}

fn take_cstring(s: String) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("invalid utf8").unwrap())
        .into_raw()
}

fn parse_difficulty_tag(tag: &str) -> Result<AiDifficulty, &'static str> {
    match tag.to_ascii_lowercase().as_str() {
        "easy" => Ok(AiDifficulty::Easy),
        "medium" => Ok(AiDifficulty::Medium),
        "hard" => Ok(AiDifficulty::Hard),
        _ => Err("unknown difficulty"),
    }
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
struct JsNode {
    x: i32,
    y: i32,
}

#[derive(Deserialize)]
struct JsEdge {
    a: usize,
    b: usize,
    #[serde(default)]
    dir: Option<String>,
}

#[derive(Deserialize)]
struct JsBoardLayout {
    width: u32,
    height: u32,
    #[serde(default)]
    r#type: Option<String>,
    #[serde(default)]
    nodes: Vec<JsNode>,
    #[serde(default)]
    edges: Vec<JsEdge>,
    #[serde(default)]
    depth: Option<u32>,
}

#[derive(Deserialize)]
struct JsConfig {
    tileset: JsTileset,
    rack_size: usize,
    board_layout: JsBoardLayout,
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

#[derive(Deserialize)]
struct JsBonusCell {
    x: i32,
    y: i32,
    #[serde(default)]
    letter_mul: Option<i8>,
    #[serde(default)]
    word_mul: Option<i8>,
    #[serde(default)]
    tags: Vec<String>,
}

/// Returns null on error; call `tt_last_error_message()` to retrieve the error string.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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
    let width = cfg.board_layout.width;
    let layer_height = cfg.board_layout.height;
    if width == 0 || layer_height == 0 {
        set_error("board dimensions must be positive");
        return std::ptr::null_mut();
    }
    let depth = cfg.board_layout.depth.unwrap_or(1);
    if depth == 0 {
        set_error("board depth must be positive");
        return std::ptr::null_mut();
    }
    let total_height = match layer_height.checked_mul(depth) {
        Some(v) => v,
        None => {
            set_error("board height * depth overflow");
            return std::ptr::null_mut();
        }
    };
    let eng_cfg = engine::GameConfig {
        tileset,
        rack_size: cfg.rack_size,
        board_layout: engine::RectBoardLayout {
            width,
            height: total_height,
        },
        ruleset_id: cfg.ruleset_id,
        dictionary_id: cfg.dictionary_id,
        rng_seed: cfg.rng_seed,
        tile_counts: cfg.tile_counts,
    };
    let mut state = match engine::GameState::new(&eng_cfg, players as usize) {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("{}", e));
            return std::ptr::null_mut();
        }
    };
    // Optional graph overlay or 3D
    if cfg.board_layout.r#type.as_deref() == Some("3d") {
        let w = match i32::try_from(width) {
            Ok(v) => v,
            Err(_) => {
                set_error("board width too large for 3D");
                return std::ptr::null_mut();
            }
        };
        let h = match i32::try_from(layer_height) {
            Ok(v) => v,
            Err(_) => {
                set_error("board height too large for 3D");
                return std::ptr::null_mut();
            }
        };
        let d = match i32::try_from(depth) {
            Ok(v) => v,
            Err(_) => {
                set_error("board depth too large for 3D");
                return std::ptr::null_mut();
            }
        };
        let mut nodes: Vec<engine::Coord2D> = Vec::new();
        for z in 0..d {
            for y in 0..h {
                for x in 0..w {
                    nodes.push(engine::Coord2D { x, y: y + z * h });
                }
            }
        }
        let index = |x: i32, y: i32, z: i32| -> usize { ((y + z * h) * w + x) as usize };
        let mut edges: Vec<(usize, usize, String)> = Vec::new();
        let mut try_edge = |x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32, tag: &str| {
            if x2 < 0 || x2 >= w || y2 < 0 || y2 >= h || z2 < 0 || z2 >= d {
                return;
            }
            edges.push((index(x1, y1, z1), index(x2, y2, z2), tag.to_string()));
        };
        for z in 0..d {
            for y in 0..h {
                for x in 0..w {
                    try_edge(x, y, z, x + 1, y, z, "X");
                    try_edge(x, y, z, x, y + 1, z, "Y");
                    try_edge(x, y, z, x, y, z + 1, "Z");
                }
            }
        }
        let ov = engine::GraphOverlay { nodes, edges };
        if let Err(e) = state.apply_graph_overlay(ov) {
            set_error(format!("{}", e));
            return std::ptr::null_mut();
        }
    } else if cfg.board_layout.r#type.as_deref() == Some("graph")
        || !cfg.board_layout.nodes.is_empty()
    {
        let nodes: Vec<engine::Coord2D> = cfg
            .board_layout
            .nodes
            .iter()
            .map(|n| engine::Coord2D { x: n.x, y: n.y })
            .collect();
        let edges: Vec<(usize, usize, String)> = cfg
            .board_layout
            .edges
            .iter()
            .map(|e| (e.a, e.b, e.dir.clone().unwrap_or_else(|| "L".into())))
            .collect();
        let ov = engine::GraphOverlay { nodes, edges };
        if let Err(e) = state.apply_graph_overlay(ov) {
            set_error(format!("{}", e));
            return std::ptr::null_mut();
        }
    }
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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_play_move(
    game: *mut GameHandle,
    placements_json: *const c_char,
) -> *mut c_char {
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
            if let Some(id) = g.state.board.geom.to_cell_id(engine::Coord2D { x, y }) {
                let cell = &g.state.board.cells[id.0 as usize];
                let s = if let Some(t) = cell.stack.last() {
                    t.kind_id.clone()
                } else {
                    String::from("")
                };
                row.push(s);
            } else {
                row.push(String::from(""));
            }
        }
        rows.push(row);
    }
    let json = serde_json::json!({"width": w, "height": h, "rows": rows});
    take_cstring(serde_json::to_string(&json).unwrap())
}

/// Returns a newly-allocated C string with current player's rack as JSON array of tiles
/// Each element: { kind_id, symbol, score }
#[no_mangle]
pub extern "C" fn tt_get_rack(game: *const GameHandle) -> *mut c_char {
    if game.is_null() {
        return std::ptr::null_mut();
    }
    let g = unsafe { &*(game as *const FfiGame) };
    let rack = &g.state.players[g.state.to_move.0].rack;
    let mut arr = Vec::with_capacity(rack.tiles.len());
    for t in &rack.tiles {
        // Map to tileset info
        let mut symbol = String::new();
        let mut score = 0i16;
        for k in &g.state.tileset.tile_kinds {
            if k.id == t.kind_id {
                symbol = k.symbol.clone();
                score = k.score;
                break;
            }
        }
        arr.push(serde_json::json!({
            "kind_id": t.kind_id,
            "symbol": symbol,
            "score": score,
        }));
    }
    take_cstring(serde_json::to_string(&arr).unwrap())
}

/// Set board bonuses from JSON array of cells: [{x,y,letter_mul?,word_mul?,tags?}]
/// Returns 1 on success, 0 on error (see tt_last_error_message()).
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_set_bonuses(game: *mut GameHandle, bonuses_json: *const c_char) -> c_uint {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    if game.is_null() {
        set_error("game is null");
        return 0;
    }
    if bonuses_json.is_null() {
        set_error("bonuses_json is null");
        return 0;
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    let cstr = unsafe { CStr::from_ptr(bonuses_json) };
    let json_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("bonuses_json is not valid UTF-8");
            return 0;
        }
    };
    let cells: Vec<JsBonusCell> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("bonuses parse error: {}", e));
            return 0;
        }
    };
    // Clear and set
    g.state.board.bonuses.clear();
    for bc in cells.into_iter() {
        let Some(id) = g
            .state
            .board
            .geom
            .to_cell_id(engine::Coord2D { x: bc.x, y: bc.y })
        else {
            // Ignore out-of-bounds silently
            continue;
        };
        let mut bonus = engine::Bonus::default();
        if let Some(lm) = bc.letter_mul {
            bonus.letter_mul = lm;
        }
        if let Some(wm) = bc.word_mul {
            bonus.word_mul = wm;
        }
        for t in bc.tags.into_iter() {
            bonus.tags.insert(t);
        }
        g.state.board.bonuses.insert(id, bonus);
    }
    1
}

/// Returns players' scores and to_move index: { players:[{score}], to_move }
#[no_mangle]
pub extern "C" fn tt_get_scores(game: *const GameHandle) -> *mut c_char {
    if game.is_null() {
        return std::ptr::null_mut();
    }
    let g = unsafe { &*(game as *const FfiGame) };
    let players: Vec<_> = g
        .state
        .players
        .iter()
        .map(|p| serde_json::json!({"score": p.score}))
        .collect();
    let val = serde_json::json!({ "players": players, "to_move": g.state.to_move.0 });
    take_cstring(serde_json::to_string(&val).unwrap())
}

// ---- Preview helpers (duplicate minimal logic for highlights) ----

fn ffi_collect_line_on_dir(
    board: &engine::Board<engine::RectGridGeometry>,
    center: engine::CellId,
    tag: &str,
    placed: &HashSet<engine::CellId>,
) -> Vec<engine::CellId> {
    let neighs: Vec<engine::CellId> = board
        .geom
        .neighbors_with_tags(center)
        .into_iter()
        .filter(|(_, t)| *t == tag)
        .map(|(n, _)| n)
        .collect();
    let mut back = center;
    if let Some(nb) = neighs.first() {
        let mut prev = center;
        let mut cur = *nb;
        loop {
            if !placed.contains(&cur) && board.cells[cur.0 as usize].stack.is_empty() {
                break;
            }
            let nxt = board
                .geom
                .neighbors_with_tags(cur)
                .into_iter()
                .filter(|(_, t)| *t == tag)
                .map(|(n, _)| n)
                .find(|n| *n != prev);
            back = cur;
            if let Some(n2) = nxt {
                prev = cur;
                cur = n2;
            } else {
                break;
            }
        }
    }
    let mut out = Vec::new();
    let mut prev = None;
    let mut cur = back;
    loop {
        if !placed.contains(&cur) && board.cells[cur.0 as usize].stack.is_empty() {
            break;
        }
        out.push(cur);
        let nxt = board
            .geom
            .neighbors_with_tags(cur)
            .into_iter()
            .filter(|(_, t)| *t == tag)
            .map(|(n, _)| n)
            .find(|n| Some(*n) != prev);
        if let Some(n2) = nxt {
            prev = Some(cur);
            cur = n2;
        } else {
            break;
        }
    }
    out
}

fn ffi_graph_find_main_path(
    board: &engine::Board<engine::RectGridGeometry>,
    placed: &HashSet<engine::CellId>,
) -> Option<(String, Vec<engine::CellId>)> {
    if placed.len() == 1 {
        let id = *placed.iter().next().unwrap();
        let tag = board
            .geom
            .neighbors_with_tags(id)
            .first()
            .map(|(_, t)| t.to_string())
            .unwrap_or_else(|| "E".into());
        return Some((tag, vec![id]));
    }
    let mut tags: HashSet<String> = HashSet::new();
    for &id in placed.iter() {
        for (n, t) in board.geom.neighbors_with_tags(id) {
            if placed.contains(&n) || !board.cells[n.0 as usize].stack.is_empty() {
                tags.insert(t.to_string());
            }
        }
    }
    // Choose a tag that connects all placed into a single path
    for tag in tags.into_iter() {
        // find starting node: a placed node with <=1 neighbor along tag
        let mut starts: Vec<engine::CellId> = Vec::new();
        for &id in placed.iter() {
            let cnt = board
                .geom
                .neighbors_with_tags(id)
                .into_iter()
                .filter(|(_, t)| *t == tag)
                .count();
            if cnt <= 1 {
                starts.push(id);
            }
        }
        let start = starts
            .first()
            .copied()
            .or_else(|| placed.iter().next().copied());
        if let Some(s) = start {
            // walk along tag and collect path
            // reuse collect logic by treating s as center and tag
            let path = ffi_collect_line_on_dir(board, s, &tag, placed);
            if !path.is_empty() {
                return Some((tag, path));
            }
        }
    }
    None
}

/// Preview a move from placements JSON. Returns JSON with validity, score, and highlight cells.
/// Schema: { valid: bool, total, main_word, main_score, cross_words, bingo, main_cells:[[x,y],...], cross_cells:[[[x,y],...],...] }
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_preview_move(
    game: *const GameHandle,
    placements_json: *const c_char,
) -> *mut c_char {
    if game.is_null() || placements_json.is_null() {
        set_error("null pointer");
        return std::ptr::null_mut();
    }
    let g = unsafe { &*(game as *const FfiGame) };
    let cstr = unsafe { CStr::from_ptr(placements_json) };
    let p_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("UTF-8 error");
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
    // Build draft and validated
    let mut mv = engine::MoveDraft { placements: vec![] };
    for p in &placements {
        let Some(cid) = g
            .state
            .board
            .geom
            .to_cell_id(engine::Coord2D { x: p.x, y: p.y })
        else {
            set_error("invalid coordinates");
            return std::ptr::null_mut();
        };
        mv.placements.push((
            cid,
            engine::Tile {
                kind_id: p.kind_id.clone(),
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
    // temp board overlay
    let mut temp_board = g.state.board.clone();
    for (cid, tile) in &validated.placements {
        temp_board.cells[cid.0 as usize].stack.push(tile.clone());
    }
    let placed_ids: HashSet<engine::CellId> =
        validated.placements.iter().map(|(id, _)| *id).collect();
    // main and cross cell paths
    let mut main_cells: Vec<(i32, i32)> = Vec::new();
    let mut cross_cells: Vec<Vec<(i32, i32)>> = Vec::new();
    if temp_board.geom.has_graph() {
        if let Some((tag, path)) = ffi_graph_find_main_path(&temp_board, &placed_ids) {
            for id in &path {
                if let Some(c) = temp_board.geom.from_cell_id(*id) {
                    main_cells.push((c.x, c.y));
                }
            }
            for (cid, _) in &validated.placements {
                let mut seen: HashSet<String> = HashSet::new();
                for (_, t) in g.state.board.geom.neighbors_with_tags(*cid) {
                    if t == tag {
                        continue;
                    }
                    if !seen.insert(t.to_string()) {
                        continue;
                    }
                    let line = ffi_collect_line_on_dir(&temp_board, *cid, t, &placed_ids);
                    if line.len() > 1 {
                        let mut vecxy = Vec::new();
                        for id in line {
                            if let Some(c) = temp_board.geom.from_cell_id(id) {
                                vecxy.push((c.x, c.y));
                            }
                        }
                        cross_cells.push(vecxy);
                    }
                }
            }
        }
    } else {
        // rect grid main path using line_is_row
        if let Some(start) = temp_board.geom.from_cell_id(validated.placements[0].0) {
            let dir = if validated.line_is_row {
                (1, 0)
            } else {
                (0, 1)
            };
            // Move to beginning
            let mut c = start;
            loop {
                let prev = engine::Coord2D {
                    x: c.x - dir.0,
                    y: c.y - dir.1,
                };
                if let Some(id) = temp_board.geom.to_cell_id(prev) {
                    if placed_ids.contains(&id) || !temp_board.cells[id.0 as usize].stack.is_empty()
                    {
                        c = prev;
                        continue;
                    }
                }
                break;
            }
            // forward collect
            loop {
                if let Some(id) = temp_board.geom.to_cell_id(c) {
                    if placed_ids.contains(&id) || !temp_board.cells[id.0 as usize].stack.is_empty()
                    {
                        main_cells.push((c.x, c.y));
                        c = engine::Coord2D {
                            x: c.x + dir.0,
                            y: c.y + dir.1,
                        };
                        continue;
                    }
                }
                break;
            }
        }
        // cross lines at each placement
        let pdir = if validated.line_is_row {
            (0, 1)
        } else {
            (1, 0)
        };
        for (cid, _) in &validated.placements {
            let center = temp_board.geom.from_cell_id(*cid).unwrap();
            // back
            let mut back = center;
            loop {
                let prev = engine::Coord2D {
                    x: back.x - pdir.0,
                    y: back.y - pdir.1,
                };
                if let Some(id) = temp_board.geom.to_cell_id(prev) {
                    if placed_ids.contains(&id) || !temp_board.cells[id.0 as usize].stack.is_empty()
                    {
                        back = prev;
                        continue;
                    }
                }
                break;
            }
            // forward collect
            let mut vecxy = Vec::new();
            let mut cur = back;
            loop {
                if let Some(id) = temp_board.geom.to_cell_id(cur) {
                    if placed_ids.contains(&id) || !temp_board.cells[id.0 as usize].stack.is_empty()
                    {
                        vecxy.push((cur.x, cur.y));
                        cur = engine::Coord2D {
                            x: cur.x + pdir.0,
                            y: cur.y + pdir.1,
                        };
                        continue;
                    }
                }
                break;
            }
            if vecxy.len() > 1 {
                cross_cells.push(vecxy);
            }
        }
    }
    // score
    let sc = g.rules.score(&g.state, &validated);
    let valid = sc.main_score >= 0;
    let val = serde_json::json!({
        "valid": valid,
        "total": sc.total,
        "main_word": sc.main_word,
        "main_score": sc.main_score,
        "cross_words": sc.cross_words,
        "bingo": sc.bingo,
        "main_cells": main_cells.iter().map(|(x,y)| vec![*x, *y]).collect::<Vec<_>>(),
        "cross_cells": cross_cells.iter().map(|v| v.iter().map(|(x,y)| vec![*x, *y]).collect::<Vec<_>>()).collect::<Vec<_>>()
    });
    take_cstring(serde_json::to_string(&val).unwrap())
}

/// Compute the engine's best move for the current player.
///
/// `difficulty` accepts "easy", "medium", or "hard".
/// If `seed_is_some` is non-zero, the `seed` value is used for deterministic randomness; otherwise RNG is disabled.
/// Returns a JSON blob describing the evaluated move or the string "null" if no moves are available.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_best_move(
    game: *mut GameHandle,
    difficulty: *const c_char,
    seed: u64,
    seed_is_some: c_uint,
) -> *mut c_char {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    if game.is_null() {
        set_error("game is null");
        return std::ptr::null_mut();
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    if difficulty.is_null() {
        set_error("difficulty is null");
        return std::ptr::null_mut();
    }
    let diff_cstr = unsafe { CStr::from_ptr(difficulty) };
    let diff_str = match diff_cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("difficulty is not valid UTF-8");
            return std::ptr::null_mut();
        }
    };
    let level = match parse_difficulty_tag(diff_str) {
        Ok(lvl) => lvl,
        Err(_) => {
            set_error("unknown difficulty");
            return std::ptr::null_mut();
        }
    };
    let mut cfg = AiConfig::for_difficulty(level);
    if seed_is_some != 0 {
        cfg.randomness = Some(seed);
    }
    let Some(eval) = engine::best_move_greedy(&g.state, &g.rules, &cfg) else {
        return take_cstring("null".to_string());
    };
    let mut placements_json = Vec::with_capacity(eval.candidate.placements.len());
    for (cid, tile) in &eval.candidate.placements {
        let Some(coord) = g.state.board.geom.from_cell_id(*cid) else {
            set_error("invalid placement coordinate");
            return std::ptr::null_mut();
        };
        let mut obj = serde_json::Map::new();
        obj.insert("x".into(), coord.x.into());
        obj.insert("y".into(), coord.y.into());
        obj.insert(
            "kind_id".into(),
            serde_json::Value::String(tile.kind_id.clone()),
        );
        if let Some(mark) = &tile.mark {
            obj.insert("mark".into(), serde_json::Value::String(mark.clone()));
        }
        placements_json.push(serde_json::Value::Object(obj));
    }
    let val = serde_json::json!({
        "word": eval.candidate.word,
        "score": eval.candidate.score,
        "total": eval.total,
        "rack_leave": eval.rack_leave,
        "board_equity": eval.board_equity,
        "endgame_penalty": eval.endgame_penalty,
        "placements": placements_json,
    });
    take_cstring(val.to_string())
}

/// Snapshot current game state as JSON string. Returns null on error.
#[no_mangle]
pub extern "C" fn tt_snapshot_state_json(game: *const GameHandle) -> *mut c_char {
    if game.is_null() {
        set_error("game is null");
        return std::ptr::null_mut();
    }
    let g = unsafe { &*(game as *const FfiGame) };
    match g.state.snapshot_json() {
        Ok(s) => take_cstring(s),
        Err(e) => {
            set_error(format!("{}", e));
            std::ptr::null_mut()
        }
    }
}

/// Restore game state from a JSON snapshot. Returns 1 on success, 0 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_restore_state_json(game: *mut GameHandle, snapshot_json: *const c_char) -> c_uint {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    if game.is_null() {
        set_error("game is null");
        return 0;
    }
    if snapshot_json.is_null() {
        set_error("snapshot_json is null");
        return 0;
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    let cstr = unsafe { CStr::from_ptr(snapshot_json) };
    let snap = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("snapshot_json is not valid UTF-8");
            return 0;
        }
    };
    match engine::GameState::from_snapshot_json(snap) {
        Ok(mut s) => {
            // Preserve dictionary (none by default for FFI) and keep rules separate
            s.dictionary = g.state.dictionary.take();
            g.state = s;
            1
        }
        Err(e) => {
            set_error(format!("{}", e));
            0
        }
    }
}

/// Free a C string previously returned by this library.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe { let _ = CString::from_raw(ptr); }
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

/// Set free_word_mode at runtime (1=true, 0=false)
#[no_mangle]
pub extern "C" fn tt_set_free_word_mode(game: *mut GameHandle, on: c_uint) {
    if game.is_null() {
        return;
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    g.rules.free_word_mode = on != 0;
}

/// Set reading direction: rtl != 0 => RTL, otherwise LTR
#[no_mangle]
pub extern "C" fn tt_set_reading_direction(game: *mut GameHandle, rtl: c_uint) {
    if game.is_null() {
        return;
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    g.rules.reading_dir = if rtl != 0 {
        engine::ReadingDirection::RTL
    } else {
        engine::ReadingDirection::LTR
    };
}

/// Configure stacking rules.
/// enabled!=0 to enable stacking; sum_scoring!=0 for sum-of-stack scoring; forbid_same!=0 to forbid same symbol overlays.
#[no_mangle]
pub extern "C" fn tt_set_stacking(
    game: *mut GameHandle,
    enabled: c_uint,
    max_height: c_uint,
    forbid_same: c_uint,
    sum_scoring: c_uint,
) {
    if game.is_null() {
        return;
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    g.rules.stacking_enabled = enabled != 0;
    g.rules.stacking_max_height = max_height as usize;
    g.rules.forbid_same_symbol_overlay = forbid_same != 0;
    g.rules.stacking_scoring = if sum_scoring != 0 {
        engine::StackScoring::SumStack
    } else {
        engine::StackScoring::TopOnly
    };
}

/// Set dictionary from text (newline-separated words). `kind` in {"fst","set","dawg","gaddag"}.
/// `case_fold` non-zero enables case-folding.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_set_dictionary_from_text(
    game: *mut GameHandle,
    kind: *const c_char,
    text: *const c_char,
    case_fold: c_uint,
) -> c_uint {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    if game.is_null() {
        set_error("game is null");
        return 0;
    }
    if kind.is_null() || text.is_null() {
        set_error("null pointer");
        return 0;
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    let kind_c = unsafe { CStr::from_ptr(kind) };
    let text_c = unsafe { CStr::from_ptr(text) };
    let kind_str = match kind_c.to_str() {
        Ok(s) => s.to_ascii_lowercase(),
        Err(_) => {
            set_error("kind is not valid UTF-8");
            return 0;
        }
    };
    let text_str = match text_c.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("text is not valid UTF-8");
            return 0;
        }
    };
    let case_fold = case_fold != 0;
    let words: Vec<String> = text_str
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let dict: Box<dyn engine::Dictionary + Send + Sync> = match kind_str.as_str() {
        "set" => Box::new(engine::SetDictionary::from_words(words, case_fold)),
        "dawg" => Box::new(engine::DawgDictionary::from_words(words, case_fold)),
        "gaddag" => {
            let opts = engine::DictionaryOptions {
                case_fold,
                ..engine::DictionaryOptions::default()
            };
            Box::new(engine::GaddagDictionary::from_words_opts(words, opts))
        }
        _ => Box::new(engine::FstDictionary::from_words(words, case_fold)),
    };
    g.state.dictionary = Some(dict);
    1
}

/// Set dictionary from FST bytes buffer.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn tt_set_dictionary_from_fst_bytes(
    game: *mut GameHandle,
    bytes: *const u8,
    len: usize,
    case_fold: c_uint,
) -> c_uint {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
    if game.is_null() {
        set_error("game is null");
        return 0;
    }
    if bytes.is_null() {
        set_error("bytes is null");
        return 0;
    }
    let g = unsafe { &mut *(game as *mut FfiGame) };
    let slice = unsafe { slice::from_raw_parts(bytes, len) };
    match engine::FstDictionary::from_bytes(slice, case_fold != 0) {
        Ok(dict) => {
            g.state.dictionary = Some(Box::new(dict));
            1
        }
        Err(e) => {
            set_error(format!("{}", e));
            0
        }
    }
}

/// Generate naive moves based on current rack. Returns JSON array of candidates.
#[no_mangle]
pub extern "C" fn tt_generate_moves(
    game: *const GameHandle,
    max_len: c_uint,
    limit: c_uint,
) -> *mut c_char {
    if game.is_null() {
        set_error("game is null");
        return std::ptr::null_mut();
    }
    let g = unsafe { &*(game as *const FfiGame) };
    let pid = g.state.to_move.0;
    let rack_kinds: Vec<String> = g.state.players[pid]
        .rack
        .tiles
        .iter()
        .map(|t| t.kind_id.clone())
        .collect();
    let cands = engine::generate_moves(&g.state, &g.rules, &rack_kinds, max_len as usize);
    let mut out = Vec::new();
    for cm in cands.into_iter().take(limit as usize) {
        let placements: Vec<serde_json::Value> = cm
            .placements
            .iter()
            .map(|(cid, t)| {
                let c = g.state.board.geom.from_cell_id(*cid).unwrap();
                serde_json::json!({ "x": c.x, "y": c.y, "kind_id": t.kind_id, "mark": t.mark })
            })
            .collect();
        out.push(serde_json::json!({ "word": cm.word, "score": cm.score, "placements": placements }));
    }
    take_cstring(serde_json::to_string(&out).unwrap())
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
        })
        .to_string()
    }

    #[test]
    fn smoke_create_get_board_and_play() {
        let cfg = CString::new(cfg_json()).unwrap();
        let game = tt_new_game(cfg.as_ptr(), 2);
        assert!(!game.is_null(), "should create game: {:?}", unsafe {
            CStr::from_ptr(tt_last_error_message())
                .to_string_lossy()
                .into_owned()
        });
        let b = tt_get_board(game);
        assert!(!b.is_null());
        tt_string_free(b);
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
            if e.is_null() {
                String::from("(none)")
            } else {
                CStr::from_ptr(e).to_string_lossy().into_owned()
            }
        });
        tt_string_free(res);
        tt_free_game(game);
    }
}
