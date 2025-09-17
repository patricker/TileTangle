use engine::{self, BoardGeometry, Rules};
use serde::Deserialize;
use serde_json::json;
use wasm_bindgen::prelude::*;

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
    let mut state = engine::GameState::new(&eng_cfg, players).map_err(to_js_err)?;
    if cfg.board_layout.r#type.as_deref() == Some("3d") {
        let w = cfg.board_layout.width as i32;
        let h = cfg.board_layout.height as i32;
        let d = cfg.board_layout.depth.unwrap_or(1) as i32;
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
    // save history snapshot (clear future on new action)
    game.future.clear();
    game.history.push(snapshot_json(game));
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
            let id = game
                .state
                .board
                .geom
                .to_cell_id(engine::Coord2D { x, y })
                .unwrap();
            let cell = &game.state.board.cells[id.0 as usize];
            let stack: Vec<serde_json::Value> = cell
                .stack
                .iter()
                .map(|t| json!({"kind_id": t.kind_id, "mark": t.mark }))
                .collect();
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
            ) {
                if let Some(tk) = game.state.tileset.tile_kinds.iter().find(|tk| tk.id == kid) {
                    *game.state.bag.counts.entry(tk.clone()).or_insert(0) = cnt as u32;
                }
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
pub fn pass_turn(game: &mut JsGame) {
    game.future.clear();
    game.history.push(snapshot_json(game));
    let pid = game.state.to_move.0;
    game.state.turn_num = game.state.turn_num.saturating_add(1);
    game.state.to_move = engine::PlayerId((pid + 1) % game.state.players.len());
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
    let n = kinds.len();
    // require enough tiles in bag
    if game.state.bag.remaining() < n as u32 {
        return Err(to_js_err("not enough tiles in bag to exchange"));
    }
    let pid = game.state.to_move.0;
    // remove from rack
    for kid in &kinds {
        // find first tile with that kind_id
        let pos = game.state.players[pid]
            .rack
            .tiles
            .iter()
            .position(|t| &t.kind_id == kid)
            .ok_or_else(|| to_js_err("tile not in rack"))?;
        let _t = game.state.players[pid].rack.tiles.remove(pos);
        // return to bag counts
        // find tilekind by id
        if let Some(tk) = game
            .state
            .tileset
            .tile_kinds
            .iter()
            .find(|tk| &tk.id == kid)
        {
            let entry = game.state.bag.counts.entry(tk.clone()).or_insert(0);
            *entry += 1;
        }
    }
    // draw same number
    for _ in 0..n {
        if let Some(t) = game.state.bag.draw_one() {
            let _ = game.state.players[pid].rack.add(t, 7);
        }
    }
    // advance turn
    pass_turn(game);
    Ok(())
}

// (set_free_word_mode defined above)
