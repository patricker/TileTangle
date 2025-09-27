use engine::{BoardGeometry, Rules};
use godot::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeSet, HashSet};
use std::convert::TryFrom;

#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct WordEngine {
    #[base]
    base: Base<RefCounted>,
    state: Option<engine::GameState>,
    rules: engine::CrosswordRules,
}

#[godot_api]
impl IRefCounted for WordEngine {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            state: None,
            rules: engine::CrosswordRules::default(),
        }
    }
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

#[godot_api]
impl WordEngine {
    /// Initialize game from a JSON config. Returns true on success.
    #[func]
    pub fn new_game(&mut self, config_json: GString, players: i64) -> bool {
        let cfg_str = config_json.to_string();
        let parsed: JsConfig = match serde_json::from_str(&cfg_str) {
            Ok(v) => v,
            Err(e) => {
                godot_error!("config parse error: {}", e);
                return false;
            }
        };
        let tileset = engine::Tileset {
            tile_kinds: parsed
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
        let width = parsed.board_layout.width;
        let layer_height = parsed.board_layout.height;
        if width == 0 || layer_height == 0 {
            godot_error!("board dimensions must be positive");
            return false;
        }
        let depth = parsed.board_layout.depth.unwrap_or(1);
        if depth == 0 {
            godot_error!("board depth must be positive");
            return false;
        }
        let total_height = match layer_height.checked_mul(depth) {
            Some(v) => v,
            None => {
                godot_error!("board height * depth overflow");
                return false;
            }
        };
        let cfg = engine::GameConfig {
            tileset,
            rack_size: parsed.rack_size,
            board_layout: engine::RectBoardLayout {
                width,
                height: total_height,
            },
            ruleset_id: parsed.ruleset_id,
            dictionary_id: parsed.dictionary_id,
            rng_seed: parsed.rng_seed,
            tile_counts: parsed.tile_counts,
        };
        match engine::GameState::new(&cfg, players.max(0) as usize) {
            Ok(mut state) => {
                // Optional graph or 3D overlay
                if parsed.board_layout.r#type.as_deref() == Some("3d") {
                    let w = match i32::try_from(width) {
                        Ok(v) => v,
                        Err(_) => {
                            godot_error!("board width too large for 3D");
                            return false;
                        }
                    };
                    let h = match i32::try_from(layer_height) {
                        Ok(v) => v,
                        Err(_) => {
                            godot_error!("board height too large for 3D");
                            return false;
                        }
                    };
                    let d = match i32::try_from(depth) {
                        Ok(v) => v,
                        Err(_) => {
                            godot_error!("board depth too large for 3D");
                            return false;
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
                    let index =
                        |x: i32, y: i32, z: i32| -> usize { ((y + z * h) * w + x) as usize };
                    let mut edges: Vec<(usize, usize, String)> = Vec::new();
                    let mut try_edge =
                        |x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32, tag: &str| {
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
                        godot_error!("{}", e);
                        return false;
                    }
                } else if parsed.board_layout.r#type.as_deref() == Some("graph")
                    || !parsed.board_layout.nodes.is_empty()
                {
                    let nodes: Vec<engine::Coord2D> = parsed
                        .board_layout
                        .nodes
                        .iter()
                        .map(|n| engine::Coord2D { x: n.x, y: n.y })
                        .collect();
                    let edges: Vec<(usize, usize, String)> = parsed
                        .board_layout
                        .edges
                        .iter()
                        .map(|e| (e.a, e.b, e.dir.clone().unwrap_or_else(|| "L".into())))
                        .collect();
                    let ov = engine::GraphOverlay { nodes, edges };
                    if let Err(e) = state.apply_graph_overlay(ov) {
                        godot_error!("{}", e);
                        return false;
                    }
                }
                self.rules.free_word_mode = parsed.free_word_mode;
                self.state = Some(state);
                true
            }
            Err(e) => {
                godot_error!("{}", e);
                false
            }
        }
    }

    /// Play a move from placements JSON. Returns score JSON or empty string on error.
    #[func]
    pub fn play_move(&mut self, placements_json: GString) -> GString {
        let st = match self.state.as_mut() {
            Some(s) => s,
            None => {
                godot_error!("game not initialized");
                return GString::from("");
            }
        };
        let p_str = placements_json.to_string();
        let placements: Vec<JsPlacement> = match serde_json::from_str(&p_str) {
            Ok(v) => v,
            Err(e) => {
                godot_error!("placements parse error: {}", e);
                return GString::from("");
            }
        };
        let mut mv = engine::MoveDraft { placements: vec![] };
        for p in placements.into_iter() {
            let cid = match st.board.geom.to_cell_id(engine::Coord2D { x: p.x, y: p.y }) {
                Some(id) => id,
                None => {
                    godot_error!("invalid coordinates");
                    return GString::from("");
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
        let rules = &self.rules;
        let validated = match rules.validate(st, &mv) {
            Ok(v) => v,
            Err(e) => {
                godot_error!("{}", e);
                return GString::from("");
            }
        };
        let score = rules.score(st, &validated);
        if score.main_score < 0 {
            godot_error!("invalid word(s)");
            return GString::from("");
        }
        if let Err(e) = rules.commit(st, validated, &score) {
            godot_error!("{}", e);
            return GString::from("");
        }
        let json = serde_json::json!({
            "total": score.total,
            "main_word": score.main_word,
            "main_score": score.main_score,
            "cross_words": score.cross_words,
            "bingo": score.bingo,
        });
        GString::from(serde_json::to_string(&json).unwrap())
    }

    /// Obtain board JSON: { width, height, rows }
    #[func]
    pub fn get_board_json(&self) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from(""),
        };
        let w = st.board.geom.width as i32;
        let h = st.board.geom.height as i32;
        let mut rows: Vec<Vec<String>> = Vec::new();
        for y in 0..h {
            let mut row = Vec::new();
            for x in 0..w {
                if let Some(id) = st.board.geom.to_cell_id(engine::Coord2D { x, y }) {
                    let cell = &st.board.cells[id.0 as usize];
                    let s = if let Some(t) = cell.stack.last() {
                        t.kind_id.clone()
                    } else {
                        String::new()
                    };
                    row.push(s);
                } else {
                    row.push(String::new());
                }
            }
            rows.push(row);
        }
        let json = serde_json::json!({"width": w, "height": h, "rows": rows});
        GString::from(serde_json::to_string(&json).unwrap())
    }

    /// Detailed board snapshot including full stacks and presence mask.
    #[func]
    pub fn get_board_cells_json(&self) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from(""),
        };
        let w = st.board.geom.width as i32;
        let h = st.board.geom.height as i32;
        let mut cells: Vec<serde_json::Value> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let coord = engine::Coord2D { x, y };
                if let Some(id) = st.board.geom.to_cell_id(coord) {
                    let cell = &st.board.cells[id.0 as usize];
                    let top = cell.stack.last().map(|t| {
                        serde_json::json!({
                            "kind_id": t.kind_id.clone(),
                            "mark": t.mark.clone(),
                        })
                    });
                    let mut stack_vec: Vec<serde_json::Value> = Vec::new();
                    for tile in &cell.stack {
                        stack_vec.push(serde_json::json!({
                            "kind_id": tile.kind_id.clone(),
                            "mark": tile.mark.clone(),
                        }));
                    }
                    cells.push(serde_json::json!({
                        "x": x,
                        "y": y,
                        "present": true,
                        "stack_height": cell.stack.len(),
                        "top": top,
                        "stack": stack_vec,
                    }));
                }
            }
        }
        let json = serde_json::json!({
            "width": w,
            "height": h,
            "graph": st.board.geom.has_graph(),
            "cells": cells,
        });
        GString::from(serde_json::to_string(&json).unwrap())
    }

    /// Toggle free-word mode for testing without a dictionary.
    #[func]
    pub fn set_free_word_mode(&mut self, on: bool) {
        self.rules.free_word_mode = on;
    }

    /// Get current player's rack as JSON array: [{kind_id, symbol, score}]
    #[func]
    pub fn get_rack_json(&self) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from(""),
        };
        let rack = &st.players[st.to_move.0].rack;
        let mut arr = Vec::with_capacity(rack.tiles.len());
        for t in &rack.tiles {
            let mut symbol = String::new();
            let mut score: i16 = 0;
            for k in &st.tileset.tile_kinds {
                if k.id == t.kind_id {
                    symbol = k.symbol.clone();
                    score = k.score;
                    break;
                }
            }
            arr.push(serde_json::json!({"kind_id": t.kind_id, "symbol": symbol, "score": score}));
        }
        GString::from(serde_json::to_string(&arr).unwrap())
    }

    /// Get scores and to_move as JSON: { players:[{score}], to_move }
    #[func]
    pub fn get_scores_json(&self) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from(""),
        };
        let players: Vec<_> = st
            .players
            .iter()
            .map(|p| serde_json::json!({"score": p.score}))
            .collect();
        let val = serde_json::json!({"players": players, "to_move": st.to_move.0});
        GString::from(serde_json::to_string(&val).unwrap())
    }

    /// Replace board bonuses from a JSON array.
    #[func]
    pub fn set_bonuses(&mut self, bonuses_json: GString) -> bool {
        let st = match self.state.as_mut() {
            Some(s) => s,
            None => return false,
        };
        let parsed: Vec<JsBonus> = match serde_json::from_str(&bonuses_json.to_string()) {
            Ok(v) => v,
            Err(e) => {
                godot_error!("bonus parse error: {}", e);
                return false;
            }
        };
        st.board.bonuses.clear();
        for b in parsed {
            let coord = engine::Coord2D { x: b.x, y: b.y };
            let Some(id) = st.board.geom.to_cell_id(coord) else {
                godot_error!("invalid bonus coord: ({}, {})", b.x, b.y);
                return false;
            };
            let mut tags = BTreeSet::new();
            for t in b.tags {
                tags.insert(t);
            }
            st.board.bonuses.insert(
                id,
                engine::Bonus {
                    letter_mul: if b.letter_mul == 0 { 1 } else { b.letter_mul },
                    word_mul: if b.word_mul == 0 { 1 } else { b.word_mul },
                    tags,
                },
            );
        }
        true
    }

    /// Return the active bonus layout as JSON array.
    #[func]
    pub fn get_bonuses_json(&self) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from(""),
        };
        let mut out: Vec<serde_json::Value> = Vec::new();
        for (id, bonus) in &st.board.bonuses {
            if let Some(coord) = st.board.geom.from_cell_id(*id) {
                let tags: Vec<String> = bonus.tags.iter().cloned().collect();
                out.push(serde_json::json!({
                    "x": coord.x,
                    "y": coord.y,
                    "letter_mul": bonus.letter_mul,
                    "word_mul": bonus.word_mul,
                    "tags": tags,
                }));
            }
        }
        GString::from(serde_json::to_string(&out).unwrap())
    }

    /// Generate candidate moves for the current rack.
    #[func]
    pub fn generate_moves(&self, max_len: i64, limit: i64) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from("[]"),
        };
        let pid = st.to_move.0;
        if pid >= st.players.len() {
            return GString::from("[]");
        }
        let rack: Vec<String> = st.players[pid]
            .rack
            .tiles
            .iter()
            .map(|t| t.kind_id.clone())
            .collect();
        let rack_default = rack.len().max(1);
        let max_len = if max_len <= 0 {
            rack_default
        } else {
            max_len as usize
        };
        let limit = if limit <= 0 { 50 } else { limit as usize };
        let candidates = engine::generate_moves(st, &self.rules, &rack, max_len);
        let mut out: Vec<serde_json::Value> = Vec::new();
        for cand in candidates.into_iter().take(limit) {
            let mut placements = Vec::new();
            for (cid, tile) in &cand.placements {
                if let Some(coord) = st.board.geom.from_cell_id(*cid) {
                    placements.push(serde_json::json!({
                        "x": coord.x,
                        "y": coord.y,
                        "kind_id": tile.kind_id.clone(),
                        "mark": tile.mark.clone(),
                    }));
                }
            }
            out.push(serde_json::json!({
                "word": cand.word,
                "score": cand.score,
                "placements": placements,
            }));
        }
        GString::from(serde_json::to_string(&out).unwrap())
    }

    /// Return recent event log entries in reverse chronological order.
    #[func]
    pub fn get_event_log_json(&self, limit: i64) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from("[]"),
        };
        let limit = if limit <= 0 { 32 } else { limit as usize };
        let mut out: Vec<serde_json::Value> = Vec::new();
        for ev in st.event_log.iter().rev().take(limit) {
            let base = serde_json::json!({
                "turn": ev.turn,
                "player": ev.player,
                "position_hash": ev.position_hash,
            });
            let entry = match &ev.kind {
                engine::GameEventKind::Play {
                    placements,
                    score,
                    total,
                } => {
                    let mut list = Vec::new();
                    for (cid, tile) in placements {
                        if let Some(coord) = st.board.geom.from_cell_id(*cid) {
                            list.push(serde_json::json!({
                                "x": coord.x,
                                "y": coord.y,
                                "kind_id": tile.kind_id.clone(),
                                "mark": tile.mark.clone(),
                            }));
                        }
                    }
                    serde_json::json!({
                        "type": "play",
                        "score": score,
                        "total": total,
                        "placements": list,
                    })
                }
                engine::GameEventKind::Draw { tiles } => serde_json::json!({
                    "type": "draw",
                    "tiles": tiles,
                }),
                engine::GameEventKind::Exchange { give, take } => serde_json::json!({
                    "type": "exchange",
                    "give": give,
                    "take": take,
                }),
                engine::GameEventKind::Pass => serde_json::json!({
                    "type": "pass",
                }),
            };
            out.push(merge_json_objects(base, entry));
        }
        GString::from(serde_json::to_string(&out).unwrap())
    }

    /// Serialize the current game state as JSON snapshot.
    #[func]
    pub fn snapshot_json(&self) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from(""),
        };
        match st.snapshot_json() {
            Ok(json) => GString::from(json),
            Err(e) => {
                godot_error!("snapshot error: {}", e);
                GString::from("")
            }
        }
    }

    /// Configure stacking/overlay rules.
    #[func]
    pub fn set_stacking(
        &mut self,
        enabled: bool,
        max_height: i64,
        forbid_same_symbol: bool,
        sum_stack_scoring: bool,
    ) {
        self.rules.stacking_enabled = enabled;
        self.rules.stacking_max_height = if max_height <= 0 {
            1
        } else {
            max_height as usize
        };
        self.rules.forbid_same_symbol_overlay = forbid_same_symbol;
        self.rules.stacking_scoring = if sum_stack_scoring {
            engine::StackScoring::SumStack
        } else {
            engine::StackScoring::TopOnly
        };
    }

    #[func]
    pub fn best_move(&self, difficulty: GString, seed: Variant) -> GString {
        let Some(state) = &self.state else {
            godot_error!("best_move called before new_game");
            return GString::from("");
        };
        let diff_tag = difficulty.to_string();
        let level = match diff_tag.to_ascii_lowercase().as_str() {
            "easy" => engine::AiDifficulty::Easy,
            "medium" => engine::AiDifficulty::Medium,
            "hard" => engine::AiDifficulty::Hard,
            _ => {
                godot_error!("unknown difficulty: {}", diff_tag);
                return GString::from("");
            }
        };
        let mut cfg = engine::AiConfig::for_difficulty(level);
        if !seed.is_nil()
            && let Ok(v) = seed.try_to::<i64>()
            && v >= 0
        {
            cfg.randomness = Some(v as u64);
        }
        let Some(eval) = engine::best_move_greedy(state, &self.rules, &cfg) else {
            return GString::from("null");
        };
        let mut placements = Vec::with_capacity(eval.candidate.placements.len());
        for (cid, tile) in &eval.candidate.placements {
            if let Some(coord) = state.board.geom.from_cell_id(*cid) {
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
                placements.push(serde_json::Value::Object(obj));
            }
        }
        let json = serde_json::json!({
            "word": eval.candidate.word,
            "score": eval.candidate.score,
            "total": eval.total,
            "rack_leave": eval.rack_leave,
            "board_equity": eval.board_equity,
            "endgame_penalty": eval.endgame_penalty,
            "placements": placements,
        });
        GString::from(json.to_string())
    }

    /// Evaluate a candidate with heuristic breakdown: returns JSON { word, score, rack_leave, board_equity, endgame_penalty, total }
    #[func]
    pub fn evaluate_candidate(&self, placements_json: GString, difficulty: GString) -> GString {
        let Some(state) = &self.state else {
            godot_error!("evaluate_candidate called before new_game");
            return GString::from("");
        };
        let mut mv = engine::MoveDraft { placements: vec![] };
        let p_str = placements_json.to_string();
        let items: Vec<JsPlacement> = match serde_json::from_str(&p_str) {
            Ok(v) => v,
            Err(_) => return GString::from(""),
        };
        for p in &items {
            if let Some(cid) = state.board.geom.to_cell_id(engine::Coord2D { x: p.x, y: p.y }) {
                mv.placements.push((cid, engine::Tile { kind_id: p.kind_id.clone(), mark: None }));
            }
        }
        let validated = match self.rules.validate(state, &mv) { Ok(v) => v, Err(_) => return GString::from("") };
        let sc = self.rules.score(state, &validated);
        if sc.main_score < 0 { return GString::from("null"); }
        let candidate = engine::CandidateMove { placements: validated.placements.clone(), word: sc.main_word.clone(), score: sc.total };
        let pid = state.to_move.0;
        let rack: Vec<String> = state.players[pid].rack.tiles.iter().map(|t| t.kind_id.clone()).collect();
        let level = match difficulty.to_string().to_ascii_lowercase().as_str() { "easy" => engine::AiDifficulty::Easy, "medium" => engine::AiDifficulty::Medium, "hard" => engine::AiDifficulty::Hard, _ => engine::AiDifficulty::Medium };
        let mut cfg = engine::AiConfig::for_difficulty(level);
        let eval = engine::evaluate_candidate_move(state, candidate, &rack, &cfg);
        let json = serde_json::json!({
            "word": eval.candidate.word,
            "score": eval.candidate.score,
            "rack_leave": eval.rack_leave,
            "board_equity": eval.board_equity,
            "endgame_penalty": eval.endgame_penalty,
            "total": eval.total,
        });
        GString::from(json.to_string())
    }

    // generate_moves already provided above

    /// Compute best move with opponent visibility option: opponent = "perfect" | "bag"
    #[func]
    pub fn best_move_with_opts(&self, difficulty: GString, seed: Variant, opponent: GString) -> GString {
        let Some(state) = &self.state else {
            godot_error!("best_move_with_opts called before new_game");
            return GString::from("");
        };
        let diff_tag = difficulty.to_string();
        let level = match diff_tag.to_ascii_lowercase().as_str() {
            "easy" => engine::AiDifficulty::Easy,
            "medium" => engine::AiDifficulty::Medium,
            "hard" => engine::AiDifficulty::Hard,
            _ => {
                godot_error!("unknown difficulty: {}", diff_tag);
                return GString::from("");
            }
        };
        let mut cfg = engine::AiConfig::for_difficulty(level);
        if !seed.is_nil()
            && let Ok(v) = seed.try_to::<i64>()
            && v >= 0
        {
            cfg.randomness = Some(v as u64);
        }
        cfg.opponent_model = match opponent.to_string().as_str() {
            "bag" => engine::OpponentModel::BagSampling,
            _ => engine::OpponentModel::PerfectInfo,
        };
        let Some(eval) = engine::best_move_greedy(state, &self.rules, &cfg) else {
            return GString::from("null");
        };
        let mut placements = Vec::with_capacity(eval.candidate.placements.len());
        for (cid, tile) in &eval.candidate.placements {
            if let Some(coord) = state.board.geom.from_cell_id(*cid) {
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
                placements.push(serde_json::Value::Object(obj));
            }
        }
        let json = serde_json::json!({
            "word": eval.candidate.word,
            "score": eval.candidate.score,
            "total": eval.total,
            "rack_leave": eval.rack_leave,
            "board_equity": eval.board_equity,
            "endgame_penalty": eval.endgame_penalty,
            "placements": placements,
        });
        GString::from(json.to_string())
    }

    /// Preview move: returns JSON { valid, total, main_word, main_score, cross_words, bingo, main_cells, cross_cells }
    #[func]
    pub fn preview_move(&self, placements_json: GString) -> GString {
        let st = match self.state.as_ref() {
            Some(s) => s,
            None => return GString::from(""),
        };
        let p_str = placements_json.to_string();
        let placements: Vec<JsPlacement> = match serde_json::from_str(&p_str) {
            Ok(v) => v,
            Err(_) => return GString::from(""),
        };
        // Build validated move
        let mut mv = engine::MoveDraft { placements: vec![] };
        for p in &placements {
            let Some(cid) = st.board.geom.to_cell_id(engine::Coord2D { x: p.x, y: p.y }) else {
                return GString::from("");
            };
            mv.placements.push((
                cid,
                engine::Tile {
                    kind_id: p.kind_id.clone(),
                    mark: None,
                },
            ));
        }
        let rules = &self.rules;
        let validated = match rules.validate(st, &mv) {
            Ok(v) => v,
            Err(_) => return GString::from(""),
        };
        let mut temp_board = st.board.clone();
        for (cid, tile) in &validated.placements {
            temp_board.cells[cid.0 as usize].stack.push(tile.clone());
        }
        use std::collections::HashSet;
        let placed_ids: HashSet<engine::CellId> =
            validated.placements.iter().map(|(id, _)| *id).collect();
        let mut main_cells: Vec<(i32, i32)> = Vec::new();
        let mut cross_cells: Vec<Vec<(i32, i32)>> = Vec::new();
        if temp_board.geom.has_graph() {
            if let Some((tag, path)) = graph_find_main_path(&temp_board, &placed_ids) {
                for id in &path {
                    if let Some(c) = temp_board.geom.from_cell_id(*id) {
                        main_cells.push((c.x, c.y));
                    }
                }
                for (cid, _) in &validated.placements {
                    let mut seen: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    for (_, t) in st.board.geom.neighbors_with_tags(*cid) {
                        if t == tag {
                            continue;
                        }
                        if !seen.insert(t.to_string()) {
                            continue;
                        }
                        let line = collect_line_on_dir(&temp_board, *cid, t, &placed_ids);
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
            if let Some(start) = temp_board.geom.from_cell_id(validated.placements[0].0) {
                let dir = if validated.line_is_row {
                    (1, 0)
                } else {
                    (0, 1)
                };
                let mut c = start;
                loop {
                    let prev = engine::Coord2D {
                        x: c.x - dir.0,
                        y: c.y - dir.1,
                    };
                    if let Some(id) = temp_board.geom.to_cell_id(prev)
                        && (placed_ids.contains(&id)
                            || !temp_board.cells[id.0 as usize].stack.is_empty())
                    {
                        c = prev;
                        continue;
                    }
                    break;
                }
                loop {
                    if let Some(id) = temp_board.geom.to_cell_id(c)
                        && (placed_ids.contains(&id)
                            || !temp_board.cells[id.0 as usize].stack.is_empty())
                    {
                        main_cells.push((c.x, c.y));
                        c = engine::Coord2D {
                            x: c.x + dir.0,
                            y: c.y + dir.1,
                        };
                        continue;
                    }
                    break;
                }
            }
            let pdir = if validated.line_is_row {
                (0, 1)
            } else {
                (1, 0)
            };
            for (cid, _) in &validated.placements {
                let center = temp_board.geom.from_cell_id(*cid).unwrap();
                let mut back = center;
                loop {
                    let prev = engine::Coord2D {
                        x: back.x - pdir.0,
                        y: back.y - pdir.1,
                    };
                    if let Some(id) = temp_board.geom.to_cell_id(prev)
                        && (placed_ids.contains(&id)
                            || !temp_board.cells[id.0 as usize].stack.is_empty())
                    {
                        back = prev;
                        continue;
                    }
                    break;
                }
                let mut vecxy = Vec::new();
                let mut cur = back;
                loop {
                    if let Some(id) = temp_board.geom.to_cell_id(cur)
                        && (placed_ids.contains(&id)
                            || !temp_board.cells[id.0 as usize].stack.is_empty())
                    {
                        vecxy.push((cur.x, cur.y));
                        cur = engine::Coord2D {
                            x: cur.x + pdir.0,
                            y: cur.y + pdir.1,
                        };
                        continue;
                    }
                    break;
                }
                if vecxy.len() > 1 {
                    cross_cells.push(vecxy);
                }
            }
        }
        let sc = rules.score(st, &validated);
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
        GString::from(serde_json::to_string(&val).unwrap())
    }
}

// Helpers duplicated from engine for GDExt preview
fn collect_line_on_dir(
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

fn graph_find_main_path(
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
    for tag in tags.into_iter() {
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
            let path = collect_line_on_dir(board, s, &tag, placed);
            if !path.is_empty() {
                return Some((tag, path));
            }
        }
    }
    None
}

fn merge_json_objects(base: serde_json::Value, extra: serde_json::Value) -> serde_json::Value {
    match (base, extra) {
        (serde_json::Value::Object(mut a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                a.insert(k, v);
            }
            serde_json::Value::Object(a)
        }
        (_, other) => other,
    }
}

struct TileTangleExtension;

#[godot::init::gdextension]
unsafe impl godot::init::ExtensionLibrary for TileTangleExtension {
    fn on_level_init(level: godot::init::InitLevel) {
        if level == godot::init::InitLevel::Scene {
            godot_print!("TileTangle Godot extension loaded");
        }
    }
}
