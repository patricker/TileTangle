use engine::{self, BoardGeometry, Rules};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::Deserialize;

#[pyclass]
struct Game {
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

#[pymethods]
impl Game {
    #[new]
    fn new(config_json: &str, players: usize) -> PyResult<Self> {
        let cfg: JsConfig = serde_json::from_str(config_json)
            .map_err(|e| PyValueError::new_err(format!("config parse error: {}", e)))?;
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
        let state = engine::GameState::new(&eng_cfg, players)
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let rules = engine::CrosswordRules {
            free_word_mode: cfg.free_word_mode,
            ..Default::default()
        };
        Ok(Self { state, rules })
    }

    fn play_move(&mut self, placements_json: &str, py: Python<'_>) -> PyResult<PyObject> {
        let placements: Vec<JsPlacement> = serde_json::from_str(placements_json)
            .map_err(|e| PyValueError::new_err(format!("placements parse error: {}", e)))?;
        let mut mv = engine::MoveDraft { placements: vec![] };
        for p in placements {
            let cid = self
                .state
                .board
                .geom
                .to_cell_id(engine::Coord2D { x: p.x, y: p.y })
                .ok_or_else(|| PyValueError::new_err("invalid coordinates"))?;
            mv.placements.push((
                cid,
                engine::Tile {
                    kind_id: p.kind_id,
                    mark: None,
                },
            ));
        }
        let validated = self
            .rules
            .validate(&self.state, &mv)
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let score = self.rules.score(&self.state, &validated);
        if score.main_score < 0 {
            return Err(PyValueError::new_err("invalid word(s)"));
        }
        self.rules
            .commit(&mut self.state, validated, &score)
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("total", score.total)?;
        dict.set_item("main_word", score.main_word)?;
        dict.set_item("main_score", score.main_score)?;
        dict.set_item("cross_words", score.cross_words)?;
        dict.set_item("bingo", score.bingo)?;
        Ok(dict.into_py(py))
    }

    fn get_board_json(&self) -> PyResult<String> {
        let w = self.state.board.geom.width as i32;
        let h = self.state.board.geom.height as i32;
        let mut rows: Vec<Vec<String>> = Vec::new();
        for y in 0..h {
            let mut row = Vec::new();
            for x in 0..w {
                let id = self
                    .state
                    .board
                    .geom
                    .to_cell_id(engine::Coord2D { x, y })
                    .unwrap();
                let cell = &self.state.board.cells[id.0 as usize];
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
        Ok(serde_json::to_string(&json).unwrap())
    }
}

#[pymodule]
fn tiletangle(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Game>()?;
    Ok(())
}

