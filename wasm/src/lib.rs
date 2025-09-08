use engine::{self, BoardGeometry, Rules};
use serde::Deserialize;
use serde_json::json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct JsGame {
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
    let state = engine::GameState::new(&eng_cfg, players).map_err(to_js_err)?;
    let rules = engine::CrosswordRules {
        free_word_mode: cfg.free_word_mode,
        ..Default::default()
    };
    Ok(JsGame { state, rules })
}

#[derive(Deserialize)]
struct JsPlacement {
    x: i32,
    y: i32,
    kind_id: String,
}

#[wasm_bindgen]
pub fn play_move(game: &mut JsGame, placements_json: &str) -> Result<JsValue, JsValue> {
    let placements: Vec<JsPlacement> = serde_json::from_str(placements_json).map_err(to_js_err)?;
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
                mark: None,
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
            let id = game
                .state
                .board
                .geom
                .to_cell_id(engine::Coord2D { x, y })
                .unwrap();
            let cell = &game.state.board.cells[id.0 as usize];
            let s = if let Some(t) = cell.stack.last() {
                t.kind_id.clone()
            } else {
                String::from("")
            };
            row.push(s);
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
