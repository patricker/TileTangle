use godot::prelude::*;
use engine::{BoardGeometry, Rules};
use serde::Deserialize;

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
        Self { base, state: None, rules: engine::CrosswordRules::default() }
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
struct JsTileset { tile_kinds: Vec<JsTileKind> }

#[derive(Deserialize)]
struct JsRectBoardLayout { width: u32, height: u32 }

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
struct JsPlacement { x: i32, y: i32, kind_id: String }

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
        let cfg = engine::GameConfig {
            tileset,
            rack_size: parsed.rack_size,
            board_layout: engine::RectBoardLayout { width: parsed.board_layout.width, height: parsed.board_layout.height },
            ruleset_id: parsed.ruleset_id,
            dictionary_id: parsed.dictionary_id,
            rng_seed: parsed.rng_seed,
            tile_counts: parsed.tile_counts,
        };
        match engine::GameState::new(&cfg, players.max(0) as usize) {
            Ok(state) => {
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
            mv.placements.push((cid, engine::Tile { kind_id: p.kind_id, mark: None }));
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
            None => return GString::from("")
        };
        let w = st.board.geom.width as i32;
        let h = st.board.geom.height as i32;
        let mut rows: Vec<Vec<String>> = Vec::new();
        for y in 0..h {
            let mut row = Vec::new();
            for x in 0..w {
                let id = st.board.geom.to_cell_id(engine::Coord2D { x, y }).unwrap();
                let cell = &st.board.cells[id.0 as usize];
                let s = if let Some(t) = cell.stack.last() { t.kind_id.clone() } else { String::new() };
                row.push(s);
            }
            rows.push(row);
        }
        let json = serde_json::json!({"width": w, "height": h, "rows": rows});
        GString::from(serde_json::to_string(&json).unwrap())
    }

    /// Toggle free-word mode for testing without a dictionary.
    #[func]
    pub fn set_free_word_mode(&mut self, on: bool) { self.rules.free_word_mode = on; }
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
