use engine::{self, AiConfig, AiDifficulty, BoardGeometry, GameState, Rules};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PyList, PyModule};
use serde::Deserialize;
use std::time::Duration;

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

fn parse_difficulty_tag(level: &str) -> PyResult<AiDifficulty> {
    match level.to_ascii_lowercase().as_str() {
        "easy" => Ok(AiDifficulty::Easy),
        "medium" | "normal" => Ok(AiDifficulty::Medium),
        "hard" => Ok(AiDifficulty::Hard),
        other => Err(PyValueError::new_err(format!(
            "unknown difficulty '{other}'"
        ))),
    }
}

impl Game {
    fn evaluated_move_to_py(
        &self,
        eval: engine::EvaluatedMove,
        py: Python<'_>,
    ) -> PyResult<PyObject> {
        let result = PyDict::new_bound(py);
        let candidate = eval.candidate;
        result.set_item("word", &candidate.word)?;
        result.set_item("score", candidate.score)?;
        result.set_item("rack_leave", eval.rack_leave)?;
        result.set_item("board_equity", eval.board_equity)?;
        result.set_item("endgame_penalty", eval.endgame_penalty)?;
        result.set_item("total", eval.total)?;
        let placements = PyList::empty_bound(py);
        for (cid, tile) in candidate.placements {
            let coord = self
                .state
                .board
                .geom
                .from_cell_id(cid)
                .ok_or_else(|| PyValueError::new_err("invalid cell id"))?;
            let pd = PyDict::new_bound(py);
            pd.set_item("x", coord.x)?;
            pd.set_item("y", coord.y)?;
            pd.set_item("kind_id", tile.kind_id)?;
            pd.set_item("mark", tile.mark)?;
            placements.append(pd)?;
        }
        result.set_item("placements", placements)?;
        Ok(result.into_py(py))
    }
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
        let mut state = engine::GameState::new(&eng_cfg, players)
            .map_err(|e| PyValueError::new_err(format!("{}", e)))?;
        let rules = engine::CrosswordRules {
            free_word_mode: cfg.free_word_mode,
            ..Default::default()
        };
        // Deal initial racks deterministically (mirrors WASM helper)
        for pid in 0..players {
            loop {
                if state.players[pid].rack.tiles.len() >= cfg.rack_size {
                    break;
                }
                if let Some(tile) = state.bag.draw_one() {
                    state.players[pid].rack.tiles.push(tile);
                } else {
                    break;
                }
            }
        }
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
        let dict = PyDict::new_bound(py);
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

    fn set_dictionary_from_words(&mut self, words: Vec<String>, case_fold: bool) -> PyResult<()> {
        let dict = engine::FstDictionary::from_words(words, case_fold);
        self.state.dictionary = Some(Box::new(dict));
        Ok(())
    }

    fn set_rack(&mut self, tiles: Vec<String>) -> PyResult<()> {
        let pid = self.state.to_move.0;
        // Return existing tiles to bag counts
        for tile in self.state.players[pid].rack.tiles.drain(..) {
            for (tk, count) in self.state.bag.counts.iter_mut() {
                if tk.id == tile.kind_id {
                    *count += 1;
                    break;
                }
            }
        }
        for kid in tiles {
            for (tk, count) in self.state.bag.counts.iter_mut() {
                if tk.id == kid {
                    if *count > 0 {
                        *count -= 1;
                    }
                    break;
                }
            }
            self.state.players[pid].rack.tiles.push(engine::Tile {
                kind_id: kid,
                mark: None,
            });
        }
        Ok(())
    }

    #[pyo3(text_signature = "(self, max_len, limit)")]
    fn generate_moves(&self, max_len: usize, limit: usize, py: Python<'_>) -> PyResult<PyObject> {
        let pid = self.state.to_move.0;
        let rack: Vec<String> = self.state.players[pid]
            .rack
            .tiles
            .iter()
            .map(|t| t.kind_id.clone())
            .collect();
        let mut cands = engine::generate_moves(&self.state, &self.rules, &rack, max_len);
        cands.sort_by(|a, b| b.score.cmp(&a.score));
        let limit = limit.min(cands.len());
        let list = PyList::empty_bound(py);
        for cm in cands.into_iter().take(limit) {
            let entry = PyDict::new_bound(py);
            entry.set_item("word", &cm.word)?;
            entry.set_item("score", cm.score)?;
            let placements = PyList::empty_bound(py);
            for (cid, tile) in cm.placements {
                let coord = self
                    .state
                    .board
                    .geom
                    .from_cell_id(cid)
                    .ok_or_else(|| PyValueError::new_err("invalid cell id"))?;
                let pd = PyDict::new_bound(py);
                pd.set_item("x", coord.x)?;
                pd.set_item("y", coord.y)?;
                pd.set_item("kind_id", &tile.kind_id)?;
                pd.set_item("mark", tile.mark.clone())?;
                placements.append(pd)?;
            }
            entry.set_item("placements", placements)?;
            list.append(entry)?;
        }
        Ok(list.into_py(py))
    }

    #[pyo3(signature = (max_len=None, lookahead_depth=None, seed=None, node_limit=None, time_limit_ms=None, difficulty=None, noise_range=None, candidate_limit=None, reply_limit=None, parallel_eval=None))]
    #[allow(clippy::too_many_arguments)]
    fn best_move_greedy(
        &self,
        max_len: Option<usize>,
        lookahead_depth: Option<usize>,
        seed: Option<u64>,
        node_limit: Option<usize>,
        time_limit_ms: Option<u64>,
        difficulty: Option<&str>,
        noise_range: Option<i32>,
        candidate_limit: Option<usize>,
        reply_limit: Option<usize>,
        parallel_eval: Option<bool>,
        py: Python<'_>,
    ) -> PyResult<Option<PyObject>> {
        let mut cfg = AiConfig::default();
        if let Some(level) = difficulty {
            let diff = parse_difficulty_tag(level)?;
            cfg.apply_difficulty(diff);
        }
        if let Some(m) = max_len {
            cfg.max_move_len = m;
        }
        if let Some(d) = lookahead_depth {
            cfg.lookahead_depth = d;
        }
        cfg.randomness = seed;
        if let Some(limit) = node_limit {
            cfg.max_nodes = Some(limit);
        }
        if let Some(ms) = time_limit_ms {
            cfg.max_duration = Some(Duration::from_millis(ms));
        }
        if let Some(noise) = noise_range {
            cfg.noise_range = noise;
        }
        if let Some(limit) = candidate_limit {
            cfg.candidate_limit = Some(limit);
        }
        if let Some(limit) = reply_limit {
            cfg.reply_move_limit = limit;
        }
        if let Some(parallel) = parallel_eval {
            cfg.parallel_eval = parallel;
        }
        let Some(eval) = engine::best_move_greedy(&self.state, &self.rules, &cfg) else {
            return Ok(None);
        };
        self.evaluated_move_to_py(eval, py).map(Some)
    }

    #[pyo3(signature = (difficulty, seed=None))]
    fn best_move(
        &self,
        difficulty: &str,
        seed: Option<u64>,
        py: Python<'_>,
    ) -> PyResult<Option<PyObject>> {
        let level = parse_difficulty_tag(difficulty)?;
        let mut cfg = AiConfig::for_difficulty(level);
        cfg.randomness = seed;
        let Some(eval) = engine::best_move_greedy(&self.state, &self.rules, &cfg) else {
            return Ok(None);
        };
        self.evaluated_move_to_py(eval, py).map(Some)
    }

    fn snapshot_json(&self) -> PyResult<String> {
        self.state
            .snapshot_json()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn snapshot_cbor(&self, py: Python<'_>) -> PyResult<PyObject> {
        let bytes = self
            .state
            .snapshot_cbor()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyBytes::new_bound(py, &bytes).into_py(py))
    }

    fn load_snapshot_json(&mut self, json: &str) -> PyResult<()> {
        let dict = self.state.dictionary.take();
        let mut restored = GameState::from_snapshot_json(json)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        restored.dictionary = dict;
        self.state = restored;
        Ok(())
    }

    fn load_snapshot_cbor(&mut self, data: &Bound<'_, PyAny>) -> PyResult<()> {
        let bytes = data.downcast::<PyBytes>()?;
        let dict = self.state.dictionary.take();
        let mut restored = GameState::from_snapshot_cbor(bytes.as_bytes())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        restored.dictionary = dict;
        self.state = restored;
        Ok(())
    }

    fn event_log(&self, py: Python<'_>) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for ev in &self.state.event_log {
            let entry = PyDict::new_bound(py);
            entry.set_item("turn", ev.turn)?;
            entry.set_item("player", ev.player)?;
            entry.set_item("position_hash", ev.position_hash)?;
            match &ev.kind {
                engine::GameEventKind::Play {
                    placements,
                    score,
                    total,
                } => {
                    entry.set_item("type", "play")?;
                    entry.set_item("score", *score)?;
                    entry.set_item("total", *total)?;
                    let placements_list = PyList::empty_bound(py);
                    for (cid, tile) in placements {
                        let coord = self
                            .state
                            .board
                            .geom
                            .from_cell_id(*cid)
                            .ok_or_else(|| PyValueError::new_err("invalid cell id"))?;
                        let pd = PyDict::new_bound(py);
                        pd.set_item("x", coord.x)?;
                        pd.set_item("y", coord.y)?;
                        pd.set_item("kind_id", tile.kind_id.clone())?;
                        pd.set_item("mark", tile.mark.clone())?;
                        placements_list.append(pd)?;
                    }
                    entry.set_item("placements", placements_list)?;
                }
                engine::GameEventKind::Draw { tiles } => {
                    entry.set_item("type", "draw")?;
                    entry.set_item("tiles", tiles.clone())?;
                }
                engine::GameEventKind::Exchange { give, take } => {
                    entry.set_item("type", "exchange")?;
                    entry.set_item("give", give.clone())?;
                    entry.set_item("take", take.clone())?;
                }
                engine::GameEventKind::Pass => {
                    entry.set_item("type", "pass")?;
                }
            }
            list.append(entry)?;
        }
        Ok(list.into_py(py))
    }
}

#[pymodule]
fn tiletangle(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Game>()?;
    Ok(())
}
