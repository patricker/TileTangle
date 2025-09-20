use console_error_panic_hook as panic_hook;
use engine::{self, AiConfig, AiDifficulty, BoardGeometry, Rules};
use js_sys::Reflect;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    panic_hook::set_once();
}

#[wasm_bindgen]
pub struct JsGame {
    state: engine::GameState,
    rules: engine::CrosswordRules,
    history: Vec<String>,
    future: Vec<String>,
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

fn parse_difficulty_tag(level: &str) -> Result<AiDifficulty, JsValue> {
    match level.to_ascii_lowercase().as_str() {
        "easy" => Ok(AiDifficulty::Easy),
        "medium" | "normal" => Ok(AiDifficulty::Medium),
        "hard" => Ok(AiDifficulty::Hard),
        other => Err(to_js_err(format!("unknown difficulty '{other}'"))),
    }
}

fn evaluated_move_to_json(game: &JsGame, eval: engine::EvaluatedMove) -> Result<String, JsValue> {
    let rack_leave = eval.rack_leave;
    let board_equity = eval.board_equity;
    let endgame_penalty = eval.endgame_penalty;
    let total = eval.total;
    let candidate = eval.candidate;
    let word = candidate.word;
    let score = candidate.score;
    let placements_vec = candidate.placements;
    let mut placements = Vec::new();
    for (cid, tile) in placements_vec {
        let coord = game
            .state
            .board
            .geom
            .from_cell_id(cid)
            .ok_or_else(|| to_js_err("invalid cell"))?;
        placements.push(json!({
            "x": coord.x,
            "y": coord.y,
            "kind_id": tile.kind_id,
            "mark": tile.mark
        }));
    }
    let payload = json!({
        "word": word,
        "score": score,
        "rack_leave": rack_leave,
        "board_equity": board_equity,
        "endgame_penalty": endgame_penalty,
        "total": total,
        "placements": placements,
    });
    Ok(serde_json::to_string(&payload).unwrap())
}

fn js_get(opts: &JsValue, key: &str) -> Result<Option<JsValue>, JsValue> {
    if !opts.is_object() {
        return Ok(None);
    }
    let val = Reflect::get(opts, &JsValue::from_str(key))
        .map_err(|_| to_js_err(format!("failed to read property '{key}'")))?;
    if val.is_undefined() || val.is_null() {
        Ok(None)
    } else {
        Ok(Some(val))
    }
}

fn js_get_string(opts: &JsValue, key: &str) -> Result<Option<String>, JsValue> {
    Ok(js_get(opts, key)?.and_then(|v| v.as_string()))
}

fn js_get_u32(opts: &JsValue, key: &str) -> Result<Option<u32>, JsValue> {
    if let Some(val) = js_get(opts, key)? {
        if let Some(n) = val.as_f64()
            && n.is_finite()
            && n >= 0.0
        {
            return Ok(Some(n as u32));
        }
        return Err(to_js_err(format!(
            "expected non-negative number for '{key}'"
        )));
    }
    Ok(None)
}

fn js_get_i32(opts: &JsValue, key: &str) -> Result<Option<i32>, JsValue> {
    if let Some(val) = js_get(opts, key)? {
        if let Some(n) = val.as_f64()
            && n.is_finite()
        {
            return Ok(Some(n as i32));
        }
        return Err(to_js_err(format!("expected number for '{key}'")));
    }
    Ok(None)
}

fn js_get_u64(opts: &JsValue, key: &str) -> Result<Option<u64>, JsValue> {
    if let Some(val) = js_get(opts, key)? {
        if let Some(n) = val.as_f64()
            && n.is_finite()
            && n >= 0.0
        {
            return Ok(Some(n as u64));
        }
        return Err(to_js_err(format!(
            "expected non-negative number for '{key}'"
        )));
    }
    Ok(None)
}

fn js_get_bool(opts: &JsValue, key: &str) -> Result<Option<bool>, JsValue> {
    if let Some(val) = js_get(opts, key)? {
        if let Some(b) = val.as_bool() {
            return Ok(Some(b));
        }
        return Err(to_js_err(format!("expected boolean for '{key}'")));
    }
    Ok(None)
}

#[wasm_bindgen]
pub fn new_game(config_json: &str, players: usize) -> Result<JsGame, JsValue> {
    let cfg: JsConfig = serde_json::from_str(config_json).map_err(to_js_err)?;
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
        return Err(to_js_err("board dimensions must be positive"));
    }
    let depth = cfg.board_layout.depth.unwrap_or(1);
    if depth == 0 {
        return Err(to_js_err("board depth must be positive"));
    }
    let total_height = layer_height
        .checked_mul(depth)
        .ok_or_else(|| to_js_err("board height * depth overflow"))?;
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
    let mut state = engine::GameState::new(&eng_cfg, players).map_err(to_js_err)?;
    if cfg.board_layout.r#type.as_deref() == Some("3d") {
        let w = i32::try_from(width).map_err(|_| to_js_err("board width too large for 3D"))?;
        let h =
            i32::try_from(layer_height).map_err(|_| to_js_err("board height too large for 3D"))?;
        let d = i32::try_from(depth).map_err(|_| to_js_err("board depth too large for 3D"))?;
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
        state.apply_graph_overlay(ov).map_err(to_js_err)?;
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
        state.apply_graph_overlay(ov).map_err(to_js_err)?;
    }
    let rules = engine::CrosswordRules {
        free_word_mode: cfg.free_word_mode,
        ..Default::default()
    };
    let mut game = JsGame {
        state,
        rules,
        history: Vec::new(),
        future: Vec::new(),
    };
    // Deal initial racks (7 tiles per player default)
    let players_n = game.state.players.len();
    for pid in 0..players_n {
        loop {
            if game.state.players[pid].rack.tiles.len() >= 7 {
                break;
            }
            if let Some(t) = game.state.bag.draw_one() {
                let _ = game.state.players[pid].rack.add(t, 7);
            } else {
                break;
            }
        }
    }
    // snapshot initial state
    let snap = snapshot_json(&game);
    game.history.push(snap);
    Ok(game)
}

#[derive(Deserialize)]
struct JsPlacement {
    x: i32,
    y: i32,
    kind_id: String,
    #[serde(default)]
    mark: Option<String>,
}

#[wasm_bindgen]
pub fn play_move(game: &mut JsGame, placements_json: &str) -> Result<JsValue, JsValue> {
    let placements: Vec<JsPlacement> = serde_json::from_str(placements_json).map_err(to_js_err)?;
    // clear redo stack on new action
    game.future.clear();
    let mut mv = engine::MoveDraft { placements: vec![] };
    for p in placements {
        let cid = game
            .state
            .board
            .geom
            .to_cell_id(engine::Coord2D { x: p.x, y: p.y })
            .ok_or_else(|| to_js_err("invalid coordinates"))?;
        mv.placements.push((
            cid,
            engine::Tile {
                kind_id: p.kind_id,
                mark: p.mark,
            },
        ));
    }
    let validated = game.rules.validate(&game.state, &mv).map_err(to_js_err)?;
    let score = game.rules.score(&game.state, &validated);
    if score.main_score < 0 {
        return Err(to_js_err("invalid word(s)"));
    }
    game.rules
        .commit(&mut game.state, validated, &score)
        .map_err(to_js_err)?;
    game.history.push(snapshot_json(game));
    Ok(JsValue::from_str(
        &serde_json::to_string(&score_to_json(&score)).unwrap(),
    ))
}

#[wasm_bindgen]
pub fn get_board(game: &JsGame) -> String {
    let w = game.state.board.geom.width as i32;
    let h = game.state.board.geom.height as i32;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for y in 0..h {
        let mut row = Vec::new();
        for x in 0..w {
            if let Some(id) = game.state.board.geom.to_cell_id(engine::Coord2D { x, y }) {
                let cell = &game.state.board.cells[id.0 as usize];
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
    serde_json::to_string(&json!({ "width": w, "height": h, "rows": rows })).unwrap()
}

fn score_to_json(sc: &engine::ScoreBreakdown) -> serde_json::Value {
    json!({
        "total": sc.total,
        "main_word": sc.main_word,
        "main_score": sc.main_score,
        "cross_words": sc.cross_words,
        "bingo": sc.bingo,
    })
}

fn to_js_err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&format!("{}", e))
}

fn snapshot_json(game: &JsGame) -> String {
    use serde_json::json;
    let w = game.state.board.geom.width as i32;
    let h = game.state.board.geom.height as i32;
    let mut rows: Vec<Vec<Vec<serde_json::Value>>> = Vec::new();
    for y in 0..h {
        let mut row: Vec<Vec<serde_json::Value>> = Vec::new();
        for x in 0..w {
            let stack = if let Some(id) = game.state.board.geom.to_cell_id(engine::Coord2D { x, y })
            {
                let cell = &game.state.board.cells[id.0 as usize];
                cell.stack
                    .iter()
                    .map(|t| json!({"kind_id": t.kind_id, "mark": t.mark }))
                    .collect()
            } else {
                Vec::new()
            };
            row.push(stack);
        }
        rows.push(row);
    }
    let racks: Vec<Vec<serde_json::Value>> = game
        .state
        .players
        .iter()
        .map(|p| {
            p.rack
                .tiles
                .iter()
                .map(|t| json!({"kind_id": t.kind_id, "mark": t.mark}))
                .collect()
        })
        .collect();
    let scores: Vec<i32> = game.state.players.iter().map(|p| p.score).collect();
    let to_move = game.state.to_move.0;
    let turn_num = game.state.turn_num;
    let bonuses: Vec<serde_json::Value> = game.state.board.bonuses.iter().map(|(cid, b)| {
        let c = game.state.board.geom.from_cell_id(*cid).unwrap();
        let tags: Vec<String> = b.tags.iter().cloned().collect();
        json!({"x": c.x, "y": c.y, "letter_mul": b.letter_mul, "word_mul": b.word_mul, "tags": tags})
    }).collect();
    let bag: Vec<serde_json::Value> = game
        .state
        .bag
        .counts
        .iter()
        .map(|(k, c)| json!({"kind_id": k.id, "count": c}))
        .collect();
    json!({"board": rows, "racks": racks, "scores": scores, "to_move": to_move, "turn_num": turn_num, "bonuses": bonuses, "bag": bag}).to_string()
}

fn restore_from_json(game: &mut JsGame, snapshot: &str) -> Result<(), JsValue> {
    let v: serde_json::Value = serde_json::from_str(snapshot).map_err(to_js_err)?;
    // restore board stacks
    let rows = v
        .get("board")
        .ok_or_else(|| to_js_err("snapshot missing board"))?
        .as_array()
        .ok_or_else(|| to_js_err("invalid board"))?;
    for (y, row) in rows.iter().enumerate() {
        let row_arr = row.as_array().ok_or_else(|| to_js_err("invalid row"))?;
        for (x, stack) in row_arr.iter().enumerate() {
            let id = game
                .state
                .board
                .geom
                .to_cell_id(engine::Coord2D {
                    x: x as i32,
                    y: y as i32,
                })
                .ok_or_else(|| to_js_err("coord OOB"))?;
            let sarr = stack.as_array().ok_or_else(|| to_js_err("invalid stack"))?;
            game.state.board.cells[id.0 as usize].stack.clear();
            for t in sarr {
                let kid = t
                    .get("kind_id")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| to_js_err("tile.kind_id"))?;
                let mark = t.get("mark").and_then(|m| {
                    if m.is_null() {
                        None
                    } else {
                        Some(m.as_str().unwrap_or("").to_string())
                    }
                });
                game.state.board.cells[id.0 as usize]
                    .stack
                    .push(engine::Tile {
                        kind_id: kid.to_string(),
                        mark,
                    });
            }
        }
    }
    // restore racks and scores
    if let Some(racks) = v.get("racks").and_then(|r| r.as_array()) {
        for (pi, r) in racks.iter().enumerate() {
            let arr = r.as_array().ok_or_else(|| to_js_err("invalid rack"))?;
            game.state.players[pi].rack.tiles.clear();
            for t in arr {
                let kid = t
                    .get("kind_id")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| to_js_err("rack.kind_id"))?;
                let mark = t.get("mark").and_then(|m| {
                    if m.is_null() {
                        None
                    } else {
                        Some(m.as_str().unwrap_or("").to_string())
                    }
                });
                game.state.players[pi].rack.tiles.push(engine::Tile {
                    kind_id: kid.to_string(),
                    mark,
                });
            }
        }
    }
    if let Some(scores) = v.get("scores").and_then(|r| r.as_array()) {
        for (pi, s) in scores.iter().enumerate() {
            game.state.players[pi].score = s.as_i64().unwrap_or(0) as i32;
        }
    }
    if let Some(tm) = v.get("to_move").and_then(|x| x.as_u64()) {
        game.state.to_move = engine::PlayerId(tm as usize);
    }
    if let Some(tn) = v.get("turn_num").and_then(|x| x.as_u64()) {
        game.state.turn_num = tn as u32;
    }
    // bonuses
    game.state.board.bonuses.clear();
    if let Some(bons) = v.get("bonuses").and_then(|x| x.as_array()) {
        use std::collections::BTreeSet;
        for b in bons {
            let x = b.get("x").and_then(|u| u.as_i64()).unwrap_or(0) as i32;
            let y = b.get("y").and_then(|u| u.as_i64()).unwrap_or(0) as i32;
            let id = game
                .state
                .board
                .geom
                .to_cell_id(engine::Coord2D { x, y })
                .ok_or_else(|| to_js_err("bonus OOB"))?;
            let letter_mul = b.get("letter_mul").and_then(|u| u.as_i64()).unwrap_or(1) as i8;
            let word_mul = b.get("word_mul").and_then(|u| u.as_i64()).unwrap_or(1) as i8;
            let mut tags = BTreeSet::new();
            if let Some(ts) = b.get("tags").and_then(|t| t.as_array()) {
                for t in ts {
                    if let Some(s) = t.as_str() {
                        tags.insert(s.to_string());
                    }
                }
            }
            game.state.board.bonuses.insert(
                id,
                engine::Bonus {
                    letter_mul,
                    word_mul,
                    tags,
                },
            );
        }
    }
    // bag counts
    if let Some(bag) = v.get("bag").and_then(|x| x.as_array()) {
        // reset all to 0
        for v in game.state.bag.counts.values_mut() {
            *v = 0;
        }
        for e in bag {
            if let (Some(kid), Some(cnt)) = (
                e.get("kind_id").and_then(|s| s.as_str()),
                e.get("count").and_then(|u| u.as_u64()),
            ) && let Some(tk) = game.state.tileset.tile_kinds.iter().find(|tk| tk.id == kid)
            {
                *game.state.bag.counts.entry(tk.clone()).or_insert(0) = cnt as u32;
            }
        }
    }
    Ok(())
}

#[wasm_bindgen]
pub fn snapshot(game: &JsGame) -> String {
    snapshot_json(game)
}

#[wasm_bindgen]
pub fn restore_snapshot(game: &mut JsGame, data: &str) -> Result<(), JsValue> {
    restore_from_json(game, data)
}

#[wasm_bindgen]
pub fn undo(game: &mut JsGame) -> Result<(), JsValue> {
    if game.history.len() <= 1 {
        return Ok(());
    }
    let curr = game.history.pop().unwrap();
    game.future.push(curr);
    let prev = game.history.last().cloned().unwrap();
    restore_from_json(game, &prev)
}

#[wasm_bindgen]
pub fn redo(game: &mut JsGame) -> Result<(), JsValue> {
    if let Some(next) = game.future.pop() {
        // push current to history
        game.history.push(next.clone());
        restore_from_json(game, &next)
    } else {
        Ok(())
    }
}

#[wasm_bindgen]
pub fn set_rack(game: &mut JsGame, rack_json: &str) -> Result<(), JsValue> {
    let tiles: Vec<String> = serde_json::from_str(rack_json).map_err(to_js_err)?;
    let pid = game.state.to_move.0;
    if tiles.len() > 21 {
        return Err(to_js_err("rack exceeds maximum capacity"));
    }
    let mut new_tiles = Vec::with_capacity(tiles.len());
    for kind_id in tiles {
        if !game
            .state
            .tileset
            .tile_kinds
            .iter()
            .any(|tk| tk.id == kind_id)
        {
            return Err(to_js_err(format!("unknown tile id '{kind_id}'")));
        }
        new_tiles.push(engine::Tile {
            kind_id,
            mark: None,
        });
    }
    game.state.players[pid].rack.tiles = new_tiles;
    Ok(())
}

#[wasm_bindgen]
pub fn set_dictionary_from_text(
    game: &mut JsGame,
    text: &str,
    case_fold: bool,
) -> Result<(), JsValue> {
    let words = collect_words_from_text(text, case_fold);
    let dict = engine::FstDictionary::from_words(words, case_fold);
    game.state.dictionary = Some(Box::new(dict));
    Ok(())
}

#[wasm_bindgen]
pub fn set_dictionary_from_fst_bytes(
    game: &mut JsGame,
    bytes: &[u8],
    case_fold: bool,
) -> Result<(), JsValue> {
    let dict = engine::FstDictionary::from_bytes(bytes, case_fold).map_err(to_js_err)?;
    game.state.dictionary = Some(Box::new(dict));
    Ok(())
}

fn collect_words_from_text(text: &str, case_fold: bool) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        let mut w = engine::nfc(s);
        if case_fold {
            w = w.to_lowercase();
        }
        words.push(w);
    }
    words
}

#[wasm_bindgen]
pub fn set_dictionary_from_text_engine(
    game: &mut JsGame,
    text: &str,
    engine_kind: &str,
    case_fold: bool,
) -> Result<(), JsValue> {
    let words = collect_words_from_text(text, case_fold);
    let kind = engine_kind.to_ascii_lowercase();
    let dict: Box<dyn engine::Dictionary + Send + Sync> = match kind.as_str() {
        "set" => Box::new(engine::SetDictionary::from_words(words.clone(), case_fold)),
        "dawg" => Box::new(engine::DawgDictionary::from_words(words.clone(), case_fold)),
        "gaddag" => {
            let opts = engine::DictionaryOptions {
                case_fold,
                ..engine::DictionaryOptions::default()
            };
            Box::new(engine::GaddagDictionary::from_words_opts(
                words.clone(),
                opts,
            ))
        }
        _ => Box::new(engine::FstDictionary::from_words(words, case_fold)),
    };
    game.state.dictionary = Some(dict);
    Ok(())
}

#[wasm_bindgen]
pub fn set_free_word_mode(game: &mut JsGame, on: bool) {
    game.rules.free_word_mode = on;
}

#[wasm_bindgen]
pub fn set_stacking(
    game: &mut JsGame,
    enabled: bool,
    max_height: u32,
    forbid_same_symbol_overlay: bool,
    sum_stack_scoring: bool,
) {
    game.rules.stacking_enabled = enabled;
    game.rules.stacking_max_height = max_height as usize;
    game.rules.forbid_same_symbol_overlay = forbid_same_symbol_overlay;
    game.rules.stacking_scoring = if sum_stack_scoring {
        engine::StackScoring::SumStack
    } else {
        engine::StackScoring::TopOnly
    };
}

#[wasm_bindgen]
pub fn set_reading_direction(game: &mut JsGame, rtl: bool) {
    game.rules.reading_dir = if rtl {
        engine::ReadingDirection::RTL
    } else {
        engine::ReadingDirection::LTR
    };
}

#[wasm_bindgen]
pub fn get_scores(game: &JsGame) -> String {
    let scores: Vec<i32> = game.state.players.iter().map(|p| p.score).collect();
    serde_json::to_string(&scores).unwrap()
}

#[wasm_bindgen]
pub fn get_rack(game: &JsGame) -> String {
    let pid = game.state.to_move.0;
    let rack: Vec<serde_json::Value> = game.state.players[pid]
        .rack
        .tiles
        .iter()
        .map(|t| serde_json::json!({ "kind_id": t.kind_id, "mark": t.mark }))
        .collect();
    serde_json::to_string(&rack).unwrap()
}

#[derive(Deserialize)]
struct JsBonus {
    x: i32,
    y: i32,
    #[serde(default)]
    letter_mul: i8,
    #[serde(default)]
    word_mul: i8,
    #[serde(default)]
    tags: Vec<String>,
}

#[wasm_bindgen]
pub fn set_bonuses(game: &mut JsGame, bonuses_json: &str) -> Result<(), JsValue> {
    let entries: Vec<JsBonus> = serde_json::from_str(bonuses_json).map_err(to_js_err)?;
    game.state.board.bonuses.clear();
    for b in entries {
        let id = game
            .state
            .board
            .geom
            .to_cell_id(engine::Coord2D { x: b.x, y: b.y })
            .ok_or_else(|| to_js_err("invalid bonus coord"))?;
        let mut tags = std::collections::BTreeSet::new();
        for t in b.tags {
            tags.insert(t);
        }
        game.state.board.bonuses.insert(
            id,
            engine::Bonus {
                letter_mul: if b.letter_mul == 0 { 1 } else { b.letter_mul },
                word_mul: if b.word_mul == 0 { 1 } else { b.word_mul },
                tags,
            },
        );
    }
    Ok(())
}

#[wasm_bindgen]
pub fn generate_moves(game: &JsGame, max_len: u32, limit: u32) -> String {
    let pid = game.state.to_move.0;
    let rack_kinds: Vec<String> = game.state.players[pid]
        .rack
        .tiles
        .iter()
        .map(|t| t.kind_id.clone())
        .collect();
    let cands = engine::generate_moves(&game.state, &game.rules, &rack_kinds, max_len as usize);
    let mut out = Vec::new();
    for cm in cands.into_iter().take(limit as usize) {
        let placements: Vec<serde_json::Value> = cm
            .placements
            .iter()
            .map(|(cid, t)| {
                let c = game.state.board.geom.from_cell_id(*cid).unwrap();
                serde_json::json!({ "x": c.x, "y": c.y, "kind_id": t.kind_id, "mark": t.mark })
            })
            .collect();
        out.push(
            serde_json::json!({ "word": cm.word, "score": cm.score, "placements": placements }),
        );
    }
    serde_json::to_string(&out).unwrap()
}

#[wasm_bindgen]
pub fn best_move_greedy(
    game: &JsGame,
    max_len: Option<u32>,
    depth: Option<u32>,
    seed: Option<u64>,
    opts: JsValue,
) -> Result<String, JsValue> {
    let mut cfg = AiConfig::default();
    if !opts.is_null() && !opts.is_undefined() {
        if let Some(level) = js_get_string(&opts, "difficulty")? {
            let diff = parse_difficulty_tag(&level)?;
            cfg.apply_difficulty(diff);
        }
        if let Some(limit) = js_get_u32(&opts, "node_limit")? {
            cfg.max_nodes = Some(limit as usize);
        }
        if let Some(ms) = js_get_u64(&opts, "time_limit_ms")? {
            cfg.max_duration = Some(Duration::from_millis(ms));
        }
        if let Some(noise) = js_get_i32(&opts, "noise_range")? {
            cfg.noise_range = noise;
        }
        if let Some(limit) = js_get_u32(&opts, "candidate_limit")? {
            cfg.candidate_limit = Some(limit as usize);
        }
        if let Some(limit) = js_get_u32(&opts, "reply_limit")? {
            cfg.reply_move_limit = limit as usize;
        }
        if let Some(par) = js_get_bool(&opts, "parallel_eval")? {
            cfg.parallel_eval = par;
        }
    }
    if let Some(m) = max_len {
        cfg.max_move_len = m as usize;
    }
    if let Some(d) = depth {
        cfg.lookahead_depth = d as usize;
    }
    cfg.randomness = seed;
    let eval = engine::best_move_greedy(&game.state, &game.rules, &cfg)
        .ok_or_else(|| to_js_err("no moves available"))?;
    evaluated_move_to_json(game, eval)
}

#[wasm_bindgen]
pub fn best_move(game: &JsGame, difficulty: &str, seed: Option<u64>) -> Result<String, JsValue> {
    let level = parse_difficulty_tag(difficulty)?;
    let mut cfg = AiConfig::for_difficulty(level);
    cfg.randomness = seed;
    let eval = engine::best_move_greedy(&game.state, &game.rules, &cfg)
        .ok_or_else(|| to_js_err("no moves available"))?;
    evaluated_move_to_json(game, eval)
}

#[wasm_bindgen]
pub fn pass_turn(game: &mut JsGame) {
    game.future.clear();
    game.state.pass_turn();
    game.history.push(snapshot_json(game));
}

#[derive(Deserialize)]
struct JsKinds {
    kinds: Vec<String>,
}

#[wasm_bindgen]
pub fn exchange_tiles(game: &mut JsGame, kinds_json: &str) -> Result<(), JsValue> {
    game.future.clear();
    game.history.push(snapshot_json(game));
    let kinds: Vec<String> = match serde_json::from_str::<Vec<String>>(kinds_json) {
        Ok(v) => v,
        Err(_) => {
            serde_json::from_str::<JsKinds>(kinds_json)
                .map_err(to_js_err)?
                .kinds
        }
    };
    game.state
        .exchange_tiles(&kinds)
        .map_err(|e| to_js_err(format!("{e}")))?;
    Ok(())
}

#[wasm_bindgen]
pub fn snapshot_state_json(game: &JsGame) -> Result<String, JsValue> {
    game.state
        .snapshot_json()
        .map_err(|e| to_js_err(format!("{e}")))
}

#[wasm_bindgen]
pub fn snapshot_state_cbor(game: &JsGame) -> Result<Vec<u8>, JsValue> {
    game.state
        .snapshot_cbor()
        .map_err(|e| to_js_err(format!("{e}")))
}

#[wasm_bindgen]
pub fn load_state_json(game: &mut JsGame, json: &str) -> Result<(), JsValue> {
    let dict = game.state.dictionary.take();
    let mut restored =
        engine::GameState::from_snapshot_json(json).map_err(|e| to_js_err(format!("{e}")))?;
    restored.dictionary = dict;
    game.state = restored;
    game.future.clear();
    game.history.clear();
    Ok(())
}

#[wasm_bindgen]
pub fn load_state_cbor(game: &mut JsGame, bytes: &[u8]) -> Result<(), JsValue> {
    let dict = game.state.dictionary.take();
    let mut restored =
        engine::GameState::from_snapshot_cbor(bytes).map_err(|e| to_js_err(format!("{e}")))?;
    restored.dictionary = dict;
    game.state = restored;
    game.future.clear();
    game.history.clear();
    Ok(())
}

#[wasm_bindgen]
pub fn get_event_log(game: &JsGame) -> Result<String, JsValue> {
    serde_json::to_string(&game.state.event_log).map_err(|e| to_js_err(format!("{e}")))
}

// (set_free_word_mode defined above)
