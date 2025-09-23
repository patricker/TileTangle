//! TileTangle Engine — Core Model (Phase 1)

use fst::Streamer;
use rand::{Rng, SeedableRng, rngs::StdRng};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
#[cfg(feature = "simd")]
use wide::i32x4;

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn zobrist_mix(seed: u64, value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    value.hash(&mut hasher);
    splitmix64(hasher.finish())
}

mod serde_cell_bonus {
    use super::{Bonus, CellId};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(value: &HashMap<CellId, Bonus>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<(u32, &Bonus)> = value.iter().map(|(cid, bonus)| (cid.0, bonus)).collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<CellId, Bonus>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<(u32, Bonus)>::deserialize(deserializer)?;
        Ok(vec
            .into_iter()
            .map(|(id, bonus)| (CellId(id), bonus))
            .collect())
    }
}

mod serde_cell_set {
    use super::CellId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashSet;

    pub fn serialize<S>(value: &Option<HashSet<CellId>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let opt: Option<Vec<u32>> = value.as_ref().map(|set| {
            let mut vec: Vec<u32> = set.iter().map(|cid| cid.0).collect();
            vec.sort_unstable();
            vec
        });
        opt.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<HashSet<CellId>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<Vec<u32>>::deserialize(deserializer)?;
        Ok(opt.map(|vec| vec.into_iter().map(CellId).collect()))
    }
}

mod serde_cell_adj {
    use super::CellId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    type RawAdjEntry = (u32, Vec<(u32, String)>);
    type RawAdjOpt = Option<Vec<RawAdjEntry>>;
    type CellAdj = Option<HashMap<CellId, Vec<(CellId, String)>>>;

    pub fn serialize<S>(value: &CellAdj, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let opt: RawAdjOpt = value.as_ref().map(|map| {
            let mut entries: Vec<RawAdjEntry> = map
                .iter()
                .map(|(cid, vec)| {
                    let inner = vec
                        .iter()
                        .map(|(other, dir)| (other.0, dir.clone()))
                        .collect();
                    (cid.0, inner)
                })
                .collect();
            entries.sort_by_key(|(id, _)| *id);
            entries
        });
        opt.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<CellAdj, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = RawAdjOpt::deserialize(deserializer)?;
        Ok(opt.map(|entries| {
            entries
                .into_iter()
                .map(|(id, vec)| {
                    (
                        CellId(id),
                        vec.into_iter()
                            .map(|(other, dir)| (CellId(other), dir))
                            .collect(),
                    )
                })
                .collect()
        }))
    }
}

mod serde_tile_counts {
    use super::TileKind;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(value: &HashMap<TileKind, u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<(TileKind, u32)> = value.iter().map(|(k, v)| (k.clone(), *v)).collect();
        vec.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<TileKind, u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<(TileKind, u32)>::deserialize(deserializer)?;
        Ok(vec.into_iter().collect())
    }
}

// -------- Error, Version, Text submodules --------
pub mod error;
pub use error::EngineError;

pub mod version;
pub use version::{EngineVersion, engine_version};

pub mod text;
pub use text::{Symbol, nfc, NormalizationMode, normalize_with_mode, Tokenizer, TokenizerRef};

// -------- Symbols & Tiles --------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileKind {
    pub id: String,
    pub symbol: Symbol,
    pub score: i16,
    pub is_blank: bool,
    pub aliases: Vec<String>,
}

impl PartialEq for TileKind {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for TileKind {}
impl Hash for TileKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub kind_id: String,
    pub mark: Option<String>,
}

// -------- Geometry (module) --------
pub mod geometry;
pub use geometry::{BoardGeometry, CellId, Coord2D, GraphOverlay, RectGridGeometry};

// -------- Board (module) --------
pub mod board;
pub use board::{Board, Bonus, Cell};

// -------- Game model (module) --------
pub mod game;
pub use game::{PlayerId, Player, GameEventKind, GameEvent, RectBoardLayout, GameConfig, MoveDraft};

// -------- Rules (plugins extracted) --------
pub mod rules;
pub use rules::plugins::{Action, UserMove, RulePlugin, BasicActionsPlugin, ScoreBonusPlugin, PluginRules};

// -------- Inventory (module) --------
pub mod inventory;
pub use inventory::{Rack, Tileset, Bag};

// -------- Players & Game State --------

// Game model moved to module

// Game events moved

// Game config moved

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub board: Board<RectGridGeometry>,
    pub players: Vec<Player>,
    pub to_move: PlayerId,
    pub bag: Bag,
    pub turn_num: u32,
    pub tileset: Tileset,
    #[serde(skip)]
    pub dictionary: Option<Box<dyn Dictionary + Send + Sync>>,
    #[serde(default)]
    pub dictionary_id: Option<String>,
    #[serde(default)]
    pub event_log: Vec<GameEvent>,
    pub zobrist_seed: u64,
}

impl Clone for GameState {
    fn clone(&self) -> Self {
        Self {
            board: self.board.clone(),
            players: self.players.clone(),
            to_move: self.to_move,
            bag: self.bag.clone(),
            turn_num: self.turn_num,
            tileset: self.tileset.clone(),
            dictionary: self.dictionary.as_ref().map(|d| d.boxed_clone()),
            dictionary_id: self.dictionary_id.clone(),
            event_log: self.event_log.clone(),
            zobrist_seed: self.zobrist_seed,
        }
    }
}

impl fmt::Debug for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameState")
            .field("board", &self.board)
            .field("players", &self.players)
            .field("to_move", &self.to_move)
            .field("bag", &self.bag)
            .field("turn_num", &self.turn_num)
            .field("tileset", &self.tileset)
            .field("dictionary_id", &self.dictionary_id)
            .field("event_log_len", &self.event_log.len())
            .field("zobrist_seed", &self.zobrist_seed)
            .finish()
    }
}

impl GameState {
    pub fn new(config: &GameConfig, players: usize) -> Result<Self, EngineError> {
        if players == 0 {
            return Err(EngineError::Config("at least 1 player"));
        }
        let geom = RectGridGeometry::new(config.board_layout.width, config.board_layout.height);
        let board = Board::new(geom);

        // Normalize symbols into tileset
        let mut kinds = Vec::with_capacity(config.tileset.tile_kinds.len());
        for tk in &config.tileset.tile_kinds {
            kinds.push(TileKind {
                id: tk.id.clone(),
                symbol: nfc(&tk.symbol),
                score: tk.score,
                is_blank: tk.is_blank,
                aliases: tk.aliases.clone(),
            });
        }
        let tileset = Tileset {
            tile_kinds: kinds.clone(),
        };
        // Build counts
        let mut counts: HashMap<TileKind, u32> = HashMap::new();
        for tk in kinds {
            let c = config.tile_counts.get(&tk.id).copied().unwrap_or(0);
            counts.insert(tk, c);
        }
        let bag = Bag::with_counts(counts, config.rng_seed);
        let zobrist_seed = splitmix64(config.rng_seed ^ 0x9E37_79B9_7F4A_7C15);
        let mut players_vec: Vec<Player> = (0..players).map(|_| Player::default()).collect();
        for p in &mut players_vec {
            // No arbitrary upper cap at the engine level; callers control sane bounds.
            p.rack_capacity = config.rack_size.max(1);
        }
        Ok(Self {
            board,
            players: players_vec,
            to_move: PlayerId(0),
            bag,
            turn_num: 0,
            tileset,
            dictionary: None,
            dictionary_id: Some(config.dictionary_id.clone()),
            event_log: Vec::new(),
            zobrist_seed,
        })
    }

    /// Place tiles without validation onto a cloned board and return it (does not mutate state)
    pub fn preview(&self, draft: &MoveDraft) -> Result<Board<RectGridGeometry>, EngineError> {
        let mut nb = self.board.clone();
        for (cid, tile) in &draft.placements {
            let idx = cid.0 as usize;
            if idx >= nb.cells.len() {
                return Err(EngineError::InvalidCell);
            }
            // For preview, simply push; collision policy (multiple tiles in same stack) allowed
            nb.cells[idx].stack.push(tile.clone());
        }
        Ok(nb)
    }

    /// Apply a graph overlay (custom adjacency and present cells) to the current rectangular geometry.
    pub fn apply_graph_overlay(&mut self, overlay: GraphOverlay) -> Result<(), EngineError> {
        self.board.geom.apply_graph_overlay(overlay)
    }

    fn push_event(&mut self, player: usize, kind: GameEventKind) {
        let hash = self.compute_position_hash();
        let turn = self.turn_num;
        self.event_log.push(GameEvent {
            turn,
            player,
            kind,
            position_hash: hash,
        });
    }

    fn advance_turn(&mut self) {
        let pid = self.to_move.0;
        self.turn_num = self.turn_num.saturating_add(1);
        if !self.players.is_empty() {
            self.to_move = PlayerId((pid + 1) % self.players.len());
        }
    }

    fn log_draw(&mut self, player: usize, tiles: &[Tile]) {
        if tiles.is_empty() {
            return;
        }
        let kinds = tiles.iter().map(|t| t.kind_id.clone()).collect();
        self.push_event(player, GameEventKind::Draw { tiles: kinds });
    }

    pub fn compute_position_hash(&self) -> u64 {
        let mut acc = 0u64;
        for (idx, cell) in self.board.cells.iter().enumerate() {
            for (depth, tile) in cell.stack.iter().enumerate() {
                acc ^= zobrist_mix(
                    self.zobrist_seed,
                    (
                        "cell",
                        idx as u32,
                        depth as u32,
                        tile.kind_id.as_str(),
                        tile.mark.as_deref(),
                    ),
                );
            }
        }
        for (pid, player) in self.players.iter().enumerate() {
            for (idx, tile) in player.rack.tiles.iter().enumerate() {
                acc ^= zobrist_mix(
                    self.zobrist_seed,
                    (
                        "rack",
                        pid as u32,
                        idx as u32,
                        tile.kind_id.as_str(),
                        tile.mark.as_deref(),
                    ),
                );
            }
            acc ^= zobrist_mix(self.zobrist_seed, ("score", pid as u32, player.score));
        }
        for (tk, count) in &self.bag.counts {
            for i in 0..*count {
                acc ^= zobrist_mix(self.zobrist_seed, ("bag", tk.id.as_str(), i));
            }
        }
        acc ^= zobrist_mix(self.zobrist_seed, ("turn", self.turn_num));
        acc ^= zobrist_mix(self.zobrist_seed, ("to_move", self.to_move.0 as u32));
        acc
    }

    pub fn pass_turn(&mut self) {
        let pid = self.to_move.0;
        self.push_event(pid, GameEventKind::Pass);
        self.advance_turn();
    }

    pub fn exchange_tiles(&mut self, kinds: &[String]) -> Result<Vec<String>, EngineError> {
        if kinds.is_empty() {
            return Ok(Vec::new());
        }
        if self.bag.remaining() < kinds.len() as u32 {
            return Err(EngineError::Config("not enough tiles in bag to exchange"));
        }
        let pid = self.to_move.0;
        let rack_size = self.players[pid].rack_size().unwrap_or(7);
        let mut give: Vec<String> = Vec::with_capacity(kinds.len());
        for kid in kinds {
            let pos = self.players[pid]
                .rack
                .tiles
                .iter()
                .position(|t| &t.kind_id == kid)
                .ok_or(EngineError::Config("tile not in rack"))?;
            let tile = self.players[pid].rack.tiles.remove(pos);
            if let Some(tk) = self
                .tileset
                .tile_kinds
                .iter()
                .find(|tk| tk.id == tile.kind_id)
            {
                *self.bag.counts.entry(tk.clone()).or_insert(0) += 1;
            }
            give.push(tile.kind_id);
        }

        let mut taken_tiles: Vec<Tile> = Vec::with_capacity(kinds.len());
        for _ in 0..kinds.len() {
            if let Some(tile) = self.bag.draw_one() {
                let clone_for_log = tile.clone();
                self.players[pid].rack.add(tile, rack_size)?;
                taken_tiles.push(clone_for_log);
            }
        }
        let take_ids: Vec<String> = taken_tiles.iter().map(|t| t.kind_id.clone()).collect();
        self.push_event(
            pid,
            GameEventKind::Exchange {
                give: give.clone(),
                take: take_ids.clone(),
            },
        );
        self.log_draw(pid, &taken_tiles);
        self.advance_turn();
        Ok(take_ids)
    }

    pub fn snapshot_json(&self) -> Result<String, EngineError> {
        serde_json::to_string(self).map_err(|e| EngineError::Serialization(e.to_string()))
    }

    pub fn snapshot_cbor(&self) -> Result<Vec<u8>, EngineError> {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(self, &mut buf)
            .map_err(|e| EngineError::Serialization(e.to_string()))?;
        Ok(buf)
    }

    pub fn from_snapshot_json(json: &str) -> Result<Self, EngineError> {
        let mut state: GameState =
            serde_json::from_str(json).map_err(|e| EngineError::Serialization(e.to_string()))?;
        state.dictionary = None;
        Ok(state)
    }

    pub fn from_snapshot_cbor(bytes: &[u8]) -> Result<Self, EngineError> {
        let cursor = std::io::Cursor::new(bytes);
        let mut state: GameState = ciborium::de::from_reader(cursor)
            .map_err(|e| EngineError::Serialization(e.to_string()))?;
        state.dictionary = None;
        Ok(state)
    }

    pub fn event_log(&self) -> &[GameEvent] {
        &self.event_log
    }
}

// Move draft moved

// -------- Rules & Scoring (Phase 2) --------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScoreBreakdown {
    pub total: i32,
    pub main_word: String,
    pub main_score: i32,
    pub cross_words: Vec<(String, i32)>,
    pub bingo: bool,
}

pub trait Rules {
    fn validate(&self, state: &GameState, draft: &MoveDraft) -> Result<ValidatedMove, EngineError>;
    fn score(&self, state: &GameState, mv: &ValidatedMove) -> ScoreBreakdown;
    fn commit(
        &self,
        state: &mut GameState,
        mv: ValidatedMove,
        score: &ScoreBreakdown,
    ) -> Result<(), EngineError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadingDirection {
    #[default]
    LTR,
    RTL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackScoring {
    #[default]
    TopOnly,
    SumStack,
}

#[derive(Debug, Clone)]
pub struct CrosswordRules {
    pub free_word_mode: bool,
    pub bingo_bonus: i32,
    pub require_center_first_move: bool,
    pub reading_dir: ReadingDirection,
    pub stacking_enabled: bool,
    pub stacking_max_height: usize,
    pub forbid_same_symbol_overlay: bool,
    pub stacking_scoring: StackScoring,
}

impl Default for CrosswordRules {
    fn default() -> Self {
        Self {
            free_word_mode: true,
            bingo_bonus: 50,
            require_center_first_move: true,
            reading_dir: ReadingDirection::LTR,
            stacking_enabled: false,
            stacking_max_height: 7,
            forbid_same_symbol_overlay: true,
            stacking_scoring: StackScoring::TopOnly,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedMove {
    pub placements: Vec<(CellId, Tile)>,
    pub line_is_row: bool,
}

impl CrosswordRules {
    pub fn center_cell(geom: &RectGridGeometry) -> CellId {
        let cx = (geom.width / 2) as i32;
        let cy = (geom.height / 2) as i32;
        geom.to_cell_id(Coord2D { x: cx, y: cy }).unwrap()
    }

    fn cell_has_tile(board: &Board<RectGridGeometry>, id: CellId) -> bool {
        !board.cells[id.0 as usize].stack.is_empty()
    }

    fn line_contiguous(board: &Board<RectGridGeometry>, ids: &[CellId], is_row: bool) -> bool {
        if ids.is_empty() {
            return false;
        }
        // sort by x or y
        let mut coords: Vec<Coord2D> = ids
            .iter()
            .map(|id| board.geom.from_cell_id(*id).unwrap())
            .collect();
        coords.sort_by(|a, b| if is_row { a.x.cmp(&b.x) } else { a.y.cmp(&b.y) });
        let fixed = if is_row { coords[0].y } else { coords[0].x };
        let (min, max) = if is_row {
            (coords.first().unwrap().x, coords.last().unwrap().x)
        } else {
            (coords.first().unwrap().y, coords.last().unwrap().y)
        };
        for k in min..=max {
            let c = if is_row {
                Coord2D { x: k, y: fixed }
            } else {
                Coord2D { x: fixed, y: k }
            };
            let id = board.geom.to_cell_id(c).unwrap();
            if !Self::cell_has_tile(board, id) && !ids.contains(&id) {
                return false;
            }
        }
        true
    }

    fn adjacent_to_existing(board: &Board<RectGridGeometry>, ids: &[CellId]) -> bool {
        for id in ids {
            // Stacking on an existing tile is inherently touching the board
            if !board.cells[id.0 as usize].stack.is_empty() {
                return true;
            }
            for n in board.geom.neighbors(*id) {
                if Self::cell_has_tile(board, n) {
                    return true;
                }
            }
        }
        false
    }

    fn tileset_lookup_kind<'a>(tileset: &'a Tileset, kind_id: &str) -> Option<&'a TileKind> {
        tileset.tile_kinds.iter().find(|tk| tk.id == kind_id)
    }

    fn tile_symbol_and_score(tileset: &Tileset, tile: &Tile) -> (i16, String) {
        if let Some(tk) = Self::tileset_lookup_kind(tileset, &tile.kind_id) {
            if tk.is_blank {
                let sym = tile.mark.clone().unwrap_or_else(|| tk.symbol.clone());
                (tk.score, sym)
            } else {
                (tk.score, tk.symbol.clone())
            }
        } else {
            (0, "?".into())
        }
    }

    fn form_word(
        board: &Board<RectGridGeometry>,
        tileset: &Tileset,
        start: Coord2D,
        dir: (i32, i32),
        placed: &std::collections::HashSet<CellId>,
        reverse_horiz: bool,
        stack_mode: StackScoring,
    ) -> (String, i32, usize) {
        // move to beginning
        let mut c = start;
        loop {
            let prev = Coord2D {
                x: c.x - dir.0,
                y: c.y - dir.1,
            };
            if let Some(id) = board.geom.to_cell_id(prev)
                && Self::cell_has_tile(board, id)
            {
                c = prev;
                continue;
            }
            break;
        }
        // build token list and score
        let mut tokens: Vec<String> = Vec::new();
        let mut score: i32 = 0;
        let mut word_mul: i32 = 1;
        let mut tiles_count: usize = 0;
        loop {
            if let Some(id) = board.geom.to_cell_id(c)
                && Self::cell_has_tile(board, id)
            {
                let cell = &board.cells[id.0 as usize];
                let tile = cell.stack.last().unwrap();
                let (ls, sym) = Self::tile_symbol_and_score(tileset, tile);
                tokens.push(sym);
                let mut letter_mul = 1i32;
                if placed.contains(&id)
                    && let Some(b) = board.bonuses.get(&id)
                {
                    letter_mul = b.letter_mul as i32;
                    word_mul *= b.word_mul as i32;
                }
                // compute contribution
                let add = match stack_mode {
                    StackScoring::TopOnly => (ls as i32) * letter_mul,
                    StackScoring::SumStack => {
                        // Sum underlying tiles (unmultiplied) + top tile with letter multiplier
                        let mut sum_under = 0i32;
                        if !board.cells[id.0 as usize].stack.is_empty() {
                            // sum all scores except top
                            for t in board.cells[id.0 as usize]
                                .stack
                                .iter()
                                .take(board.cells[id.0 as usize].stack.len().saturating_sub(1))
                            {
                                let (s_u, _) = Self::tile_symbol_and_score(tileset, t);
                                sum_under += s_u as i32;
                            }
                        }
                        sum_under + (ls as i32) * letter_mul
                    }
                };
                score += add;
                tiles_count += 1;
                c = Coord2D {
                    x: c.x + dir.0,
                    y: c.y + dir.1,
                };
                continue;
            }
            break;
        }
        let word = if reverse_horiz {
            tokens.into_iter().rev().collect::<Vec<_>>().join("")
        } else {
            tokens.join("")
        };
        (word, score * word_mul, tiles_count)
    }

    fn graph_adjacent_to_existing(board: &Board<RectGridGeometry>, ids: &HashSet<CellId>) -> bool {
        for id in ids {
            if !board.cells[id.0 as usize].stack.is_empty() {
                return true;
            }
            for n in board.geom.neighbors(*id) {
                if !ids.contains(&n) && Self::cell_has_tile(board, n) {
                    return true;
                }
            }
        }
        false
    }

    fn graph_find_main_dir_and_path(
        board: &Board<RectGridGeometry>,
        placed: &HashSet<CellId>,
    ) -> Option<(String, Vec<CellId>)> {
        // Single tile: choose arbitrary tag and path of length 1
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
        // Collect candidate tags between placed neighbors
        let mut tags: HashSet<String> = HashSet::new();
        for &id in placed.iter() {
            for (n, t) in board.geom.neighbors_with_tags(id) {
                if placed.contains(&n) || CrosswordRules::cell_has_tile(board, n) {
                    tags.insert(t.to_string());
                }
            }
        }
        // Try each tag
        for tag in tags.into_iter() {
            // endpoints among placed nodes using only this tag
            let mut deg: HashMap<CellId, usize> = HashMap::new();
            for &u in placed.iter() {
                let mut d = 0usize;
                for (v, t) in board.geom.neighbors_with_tags(u) {
                    if t == tag && placed.contains(&v) {
                        d += 1;
                    }
                }
                deg.insert(u, d);
            }
            let endpoints: Vec<CellId> = deg
                .iter()
                .filter_map(|(k, d)| if *d <= 1 { Some(*k) } else { None })
                .collect();
            if endpoints.is_empty() || endpoints.len() > 2 {
                continue;
            }
            let start = endpoints[0];
            let goal = if endpoints.len() == 2 {
                endpoints[1]
            } else {
                start
            };
            if let Some(path) = bfs_path_on_dir(board, start, goal, &tag, placed) {
                return Some((tag, path));
            }
        }
        None
    }
}

impl Rules for CrosswordRules {
    fn validate(&self, state: &GameState, draft: &MoveDraft) -> Result<ValidatedMove, EngineError> {
        if draft.placements.is_empty() {
            return Err(EngineError::Config("no tiles placed"));
        }
        if state.board.geom.has_graph() {
            // Graph-based: collisions
            for (cid, tile) in &draft.placements {
                let occupied = Self::cell_has_tile(&state.board, *cid);
                if occupied {
                    if !self.stacking_enabled {
                        return Err(EngineError::Collision(*cid));
                    }
                    let cell = &state.board.cells[cid.0 as usize];
                    if cell.stack.len() + 1 > self.stacking_max_height {
                        return Err(EngineError::Config("stack too high"));
                    }
                    if self.forbid_same_symbol_overlay
                        && let Some(top) = cell.stack.last()
                    {
                        let (_, top_sym) = Self::tile_symbol_and_score(&state.tileset, top);
                        let (_, new_sym) = Self::tile_symbol_and_score(&state.tileset, tile);
                        if top_sym == new_sym {
                            return Err(EngineError::Config("cannot overlay same symbol"));
                        }
                    }
                }
            }
            let ids_set: HashSet<CellId> = draft.placements.iter().map(|(id, _)| *id).collect();
            // Anchor unless board empty
            let any_on_board = state.board.cells.iter().any(|c| !c.stack.is_empty());
            if any_on_board && !Self::graph_adjacent_to_existing(&state.board, &ids_set) {
                return Err(EngineError::Config("must connect to existing tiles"));
            }
            // Line and contiguity along a single direction tag
            if Self::graph_find_main_dir_and_path(&state.board, &ids_set).is_none() {
                return Err(EngineError::Config("must be straight line"));
            }
            return Ok(ValidatedMove {
                placements: draft.placements.clone(),
                line_is_row: true,
            });
        }
        // no collisions: cannot place on occupied cells (unless stacking is enabled)
        for (cid, tile) in &draft.placements {
            let occupied = CrosswordRules::cell_has_tile(&state.board, *cid);
            if occupied {
                if !self.stacking_enabled {
                    return Err(EngineError::Collision(*cid));
                }
                let cell = &state.board.cells[cid.0 as usize];
                if cell.stack.len() + 1 > self.stacking_max_height {
                    return Err(EngineError::Config("stack too high"));
                }
                if self.forbid_same_symbol_overlay
                    && let Some(top) = cell.stack.last()
                {
                    let (_, top_sym) = Self::tile_symbol_and_score(&state.tileset, top);
                    let (_, new_sym) = Self::tile_symbol_and_score(&state.tileset, tile);
                    if top_sym == new_sym {
                        return Err(EngineError::Config("cannot overlay same symbol"));
                    }
                }
            }
        }
        // all in single row or column
        let coords: Vec<Coord2D> = draft
            .placements
            .iter()
            .map(|(id, _)| state.board.geom.from_cell_id(*id).unwrap())
            .collect();
        let same_row = coords.iter().all(|c| c.y == coords[0].y);
        let same_col = coords.iter().all(|c| c.x == coords[0].x);
        if !same_row && !same_col {
            return Err(EngineError::Config("must be straight line"));
        }
        let is_row = same_row;
        // contiguity across span
        let ids: Vec<CellId> = draft.placements.iter().map(|(id, _)| *id).collect();
        if !Self::line_contiguous(&state.board, &ids, is_row) {
            return Err(EngineError::Config("not contiguous"));
        }
        // first move center
        let any_on_board = state.board.cells.iter().any(|c| !c.stack.is_empty());
        if !any_on_board {
            if self.require_center_first_move {
                let center = Self::center_cell(&state.board.geom);
                if !ids.contains(&center) {
                    return Err(EngineError::Config("first move must cover center"));
                }
            }
        } else {
            // must touch existing tiles
            if !Self::adjacent_to_existing(&state.board, &ids) {
                return Err(EngineError::Config("must connect to existing tiles"));
            }
        }
        Ok(ValidatedMove {
            placements: draft.placements.clone(),
            line_is_row: is_row,
        })
    }

    fn score(&self, state: &GameState, mv: &ValidatedMove) -> ScoreBreakdown {
        if state.board.geom.has_graph() {
            use std::collections::HashSet as Hs;
            let placed_ids: Hs<CellId> = mv.placements.iter().map(|(id, _)| *id).collect();
            // Overlay placements for scoring
            let mut temp_board = state.board.clone();
            for (cid, tile) in &mv.placements {
                temp_board.cells[cid.0 as usize].stack.push(tile.clone());
            }
            let mut main_word = String::new();
            let mut main_score = 0;
            let mut cross_words: Vec<(String, i32)> = Vec::new();
            if let Some((tag, path)) =
                CrosswordRules::graph_find_main_dir_and_path(&state.board, &placed_ids)
            {
                let (w, s) = score_word_on_path(
                    &temp_board,
                    &state.tileset,
                    &path,
                    &placed_ids,
                    self.stacking_scoring,
                );
                main_word = w;
                main_score = s;
                for (cid, _) in &mv.placements {
                    // unique other dir tags at this node
                    let mut seen: HashSet<String> = HashSet::new();
                    for (_, t) in state.board.geom.neighbors_with_tags(*cid) {
                        if t == tag {
                            continue;
                        }
                        if !seen.insert(t.to_string()) {
                            continue;
                        }
                        let line = collect_line_on_dir(&temp_board, *cid, t, &placed_ids);
                        if line.len() > 1 {
                            let (cw, cs) = score_word_on_path(
                                &temp_board,
                                &state.tileset,
                                &line,
                                &placed_ids,
                                self.stacking_scoring,
                            );
                            cross_words.push((cw, cs));
                        }
                    }
                }
            }
            let total = main_score + cross_words.iter().map(|(_, s)| *s).sum::<i32>();
            // dictionary checks if enabled
            if !self.free_word_mode
                && let Some(dict) = &state.dictionary
            {
                if !dict.contains(&main_word) {
                    return ScoreBreakdown {
                        total: -1,
                        main_word,
                        main_score: -1,
                        cross_words: vec![],
                        bingo: false,
                    };
                }
                for (w, _) in &cross_words {
                    if !dict.contains(w) {
                        return ScoreBreakdown {
                            total: -1,
                            main_word,
                            main_score: -1,
                            cross_words: vec![],
                            bingo: false,
                        };
                    }
                }
            }
            return ScoreBreakdown {
                total,
                main_word,
                main_score,
                cross_words,
                bingo: false,
            };
        }
        // Build set of placed cells
        let placed_ids: std::collections::HashSet<CellId> =
            mv.placements.iter().map(|(id, _)| *id).collect();
        // Temporarily overlay tiles to compute words
        let mut temp_board = state.board.clone();
        for (cid, tile) in &mv.placements {
            temp_board.cells[cid.0 as usize].stack.push(tile.clone());
        }
        // main word direction
        let start_coord = temp_board.geom.from_cell_id(mv.placements[0].0).unwrap();
        let dir = if mv.line_is_row { (1, 0) } else { (0, 1) };
        let reverse_main = self.reading_dir == ReadingDirection::RTL && dir.1 == 0;
        let (main_word, main_score, _main_tiles) = Self::form_word(
            &temp_board,
            &state.tileset,
            start_coord,
            dir,
            &placed_ids,
            reverse_main,
            self.stacking_scoring,
        );
        // cross words
        let pdir = if mv.line_is_row { (0, 1) } else { (1, 0) };
        let mut cross_words = Vec::new();
        for (cid, _) in &mv.placements {
            let c = temp_board.geom.from_cell_id(*cid).unwrap();
            // Only if neighbors in perpendicular direction form a word length > 1
            // Build word centered at c in pdir
            let reverse_cross = self.reading_dir == ReadingDirection::RTL && pdir.1 == 0;
            let (w, s, t) = Self::form_word(
                &temp_board,
                &state.tileset,
                c,
                pdir,
                &placed_ids,
                reverse_cross,
                self.stacking_scoring,
            );
            if t > 1 {
                cross_words.push((w, s));
            }
        }
        let mut total = main_score + cross_words.iter().map(|(_, s)| *s).sum::<i32>();
        // dictionary checks if enabled
        if !self.free_word_mode
            && let Some(dict) = &state.dictionary
        {
            if !dict.contains(&main_word) {
                return ScoreBreakdown {
                    total: -1,
                    main_word,
                    main_score: -1,
                    cross_words: vec![],
                    bingo: false,
                };
            }
            for (w, _) in &cross_words {
                if !dict.contains(w) {
                    return ScoreBreakdown {
                        total: -1,
                        main_word,
                        main_score: -1,
                        cross_words: vec![],
                        bingo: false,
                    };
                }
            }
        }
        // dictionary checks if enabled
        if !self.free_word_mode
            && let Some(dict) = &state.dictionary
        {
            if !dict.contains(&main_word) {
                return ScoreBreakdown {
                    total: -1,
                    main_word,
                    main_score: -1,
                    cross_words: vec![],
                    bingo: false,
                };
            }
            for (w, _) in &cross_words {
                if !dict.contains(w) {
                    return ScoreBreakdown {
                        total: -1,
                        main_word,
                        main_score: -1,
                        cross_words: vec![],
                        bingo: false,
                    };
                }
            }
        }
        // bingo
        let bingo = mv.placements.len() >= state.players[state.to_move.0].rack.tiles.len()
            && !mv.placements.is_empty()
            && state.players[state.to_move.0].rack.len() >= 7; // heuristic
        let bingo = if bingo {
            total += self.bingo_bonus;
            true
        } else {
            false
        };
        ScoreBreakdown {
            total,
            main_word,
            main_score,
            cross_words,
            bingo,
        }
    }

    fn commit(
        &self,
        state: &mut GameState,
        mv: ValidatedMove,
        score: &ScoreBreakdown,
    ) -> Result<(), EngineError> {
        // Place tiles
        for (cid, tile) in &mv.placements {
            state.board.cells[cid.0 as usize].stack.push(tile.clone());
        }
        // Update score
        let pid = state.to_move.0;
        state.players[pid].score += score.total;
        // Remove used tiles from rack (by kind_id occurrences)
        let mut used_counts: HashMap<String, usize> = HashMap::new();
        for (_, t) in &mv.placements {
            *used_counts.entry(t.kind_id.clone()).or_default() += 1;
        }
        let mut new_rack = Vec::new();
        for t in state.players[pid].rack.tiles.drain(..) {
            if let Some(entry) = used_counts.get_mut(&t.kind_id)
                && *entry > 0
            {
                *entry -= 1;
                continue;
            }
            new_rack.push(t);
        }
        state.players[pid].rack.tiles = new_rack;
        // Refill rack
        let want = state.players[pid].rack.tiles.len();
        let draw_n = (state.players[pid].rack_size().unwrap_or(7)).saturating_sub(want); // default 7
        let mut drawn = state.bag.draw(draw_n);
        let drawn_clone = drawn.clone();
        state.players[pid].rack.tiles.append(&mut drawn);

        state.push_event(
            pid,
            GameEventKind::Play {
                placements: mv.placements.clone(),
                score: score.main_score,
                total: score.total,
            },
        );
        state.log_draw(pid, &drawn_clone);
        state.advance_turn();
        Ok(())
    }
}

// Rule plugins moved to rules::plugins
// ---- Graph helpers ----

fn bfs_path_on_dir(
    board: &Board<RectGridGeometry>,
    start: CellId,
    goal: CellId,
    tag: &str,
    placed: &HashSet<CellId>,
) -> Option<Vec<CellId>> {
    use std::collections::VecDeque;
    let mut q = VecDeque::new();
    let mut prev: HashMap<CellId, Option<CellId>> = HashMap::new();
    let mut seen: HashSet<CellId> = HashSet::new();
    q.push_back(start);
    seen.insert(start);
    prev.insert(start, None);
    while let Some(u) = q.pop_front() {
        if u == goal {
            break;
        }
        for (v, t) in board.geom.neighbors_with_tags(u) {
            if t != tag {
                continue;
            }
            // allow traversal only through placed nodes or existing tiles
            if !placed.contains(&v) && board.cells[v.0 as usize].stack.is_empty() {
                continue;
            }
            if seen.insert(v) {
                prev.insert(v, Some(u));
                q.push_back(v);
            }
        }
    }
    if !prev.contains_key(&goal) {
        return None;
    }
    // reconstruct path
    let mut path = Vec::new();
    let mut cur = goal;
    path.push(cur);
    while let Some(Some(p)) = prev.get(&cur) {
        cur = *p;
        path.push(cur);
    }
    path.reverse();
    Some(path)
}

fn collect_line_on_dir(
    board: &Board<RectGridGeometry>,
    center: CellId,
    tag: &str,
    placed: &HashSet<CellId>,
) -> Vec<CellId> {
    // find one neighbor to go backwards
    let neighs: Vec<CellId> = board
        .geom
        .neighbors_with_tags(center)
        .into_iter()
        .filter(|(_, t)| *t == tag)
        .map(|(n, _)| n)
        .collect();
    let mut back = center;
    // choose a backward direction arbitrarily among available
    if let Some(nb) = neighs.first() {
        let mut prev = center;
        let mut cur = *nb;
        // walk backwards as long as nodes are filled (placed or existing)
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
    // now walk forward from `back` accumulating cells
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

fn score_word_on_path(
    board: &Board<RectGridGeometry>,
    tileset: &Tileset,
    path: &[CellId],
    placed: &HashSet<CellId>,
    stack_mode: StackScoring,
) -> (String, i32) {
    let mut tokens: Vec<String> = Vec::new();
    let mut scalar_score: i32 = 0;
    let mut word_mul: i32 = 1;
    #[cfg(feature = "simd")]
    let mut top_only_scores: Vec<i32> = Vec::with_capacity(path.len());
    #[cfg(feature = "simd")]
    let mut top_only_multipliers: Vec<i32> = Vec::with_capacity(path.len());
    for id in path {
        let cell = &board.cells[id.0 as usize];
        if let Some(tile) = cell.stack.last() {
            let (ls, sym) = CrosswordRules::tile_symbol_and_score(tileset, tile);
            tokens.push(sym);
            let mut letter_mul = 1i32;
            if placed.contains(id)
                && let Some(b) = board.bonuses.get(id)
            {
                letter_mul = b.letter_mul as i32;
                word_mul *= b.word_mul as i32;
            }
            match stack_mode {
                StackScoring::TopOnly => {
                    #[cfg(feature = "simd")]
                    {
                        top_only_scores.push(ls as i32);
                        top_only_multipliers.push(letter_mul);
                    }
                    #[cfg(not(feature = "simd"))]
                    {
                        scalar_score += (ls as i32) * letter_mul;
                    }
                }
                StackScoring::SumStack => {
                    let mut sum_under = 0i32;
                    if cell.stack.len() > 1 {
                        for t in cell.stack.iter().take(cell.stack.len() - 1) {
                            let (s_u, _) = CrosswordRules::tile_symbol_and_score(tileset, t);
                            sum_under += s_u as i32;
                        }
                    }
                    scalar_score += sum_under + (ls as i32) * letter_mul;
                }
            }
        }
    }
    let score = match stack_mode {
        StackScoring::TopOnly => {
            #[cfg(feature = "simd")]
            {
                simd_dot_product(&top_only_scores, &top_only_multipliers)
            }
            #[cfg(not(feature = "simd"))]
            {
                scalar_score
            }
        }
        StackScoring::SumStack => scalar_score,
    };
    (tokens.join(""), score * word_mul)
}

#[cfg(feature = "simd")]
fn simd_dot_product(lhs: &[i32], rhs: &[i32]) -> i32 {
    debug_assert_eq!(lhs.len(), rhs.len());
    let len = lhs.len().min(rhs.len());
    let mut total = 0i32;
    let mut i = 0usize;
    while i + 4 <= len {
        let a = i32x4::from([lhs[i], lhs[i + 1], lhs[i + 2], lhs[i + 3]]);
        let b = i32x4::from([rhs[i], rhs[i + 1], rhs[i + 2], rhs[i + 3]]);
        let prod = (a * b).to_array();
        total += prod[0] + prod[1] + prod[2] + prod[3];
        i += 4;
    }
    while i < len {
        total += lhs[i] * rhs[i];
        i += 1;
    }
    total
}
// -------- Move Generation (Phase 12 start) --------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateMove {
    pub placements: Vec<(CellId, Tile)>,
    pub word: String,
    pub score: i32,
}

// -------- AI (Phase 13) --------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub rack_leave: HashMap<String, i32>,
    pub randomness: Option<u64>,
    pub max_move_len: usize,
    pub lookahead_depth: usize,
    pub max_nodes: Option<usize>,
    pub max_duration: Option<Duration>,
    pub candidate_limit: Option<usize>,
    pub reply_move_limit: usize,
    pub noise_range: i32,
    pub parallel_eval: bool,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            rack_leave: default_rack_leave_table(),
            randomness: None,
            max_move_len: 15,
            lookahead_depth: 0,
            max_nodes: None,
            max_duration: None,
            candidate_limit: None,
            reply_move_limit: usize::MAX,
            noise_range: 0,
            parallel_eval: false,
        }
    }
}

impl AiConfig {
    fn requires_rng(&self) -> bool {
        self.noise_range > 0
    }

    pub fn for_difficulty(level: AiDifficulty) -> Self {
        let mut cfg = Self::default();
        cfg.apply_difficulty(level);
        cfg
    }

    pub fn apply_difficulty(&mut self, level: AiDifficulty) {
        match level {
            AiDifficulty::Easy => {
                self.lookahead_depth = 0;
                self.max_nodes = Some(16);
                self.max_duration = Some(Duration::from_millis(5));
                self.candidate_limit = Some(20);
                self.reply_move_limit = 6;
                self.noise_range = 12;
                self.parallel_eval = false;
            }
            AiDifficulty::Medium => {
                self.lookahead_depth = 0;
                self.max_nodes = Some(64);
                self.max_duration = Some(Duration::from_millis(25));
                self.candidate_limit = Some(32);
                self.reply_move_limit = 12;
                self.noise_range = 4;
                self.parallel_eval = false;
            }
            AiDifficulty::Hard => {
                self.lookahead_depth = 1;
                self.max_nodes = None;
                self.max_duration = None;
                self.candidate_limit = None;
                self.reply_move_limit = usize::MAX;
                self.noise_range = 0;
                self.parallel_eval = false;
            }
        }
    }
}

fn default_rack_leave_table() -> HashMap<String, i32> {
    HashMap::from([
        ("A".into(), 1),
        ("E".into(), 1),
        ("I".into(), 1),
        ("L".into(), 1),
        ("N".into(), 1),
        ("R".into(), 1),
        ("S".into(), 1),
        ("T".into(), 1),
        ("O".into(), 0),
        ("D".into(), -1),
        ("G".into(), -1),
        ("B".into(), -1),
        ("M".into(), -1),
        ("P".into(), -1),
        ("C".into(), -1),
        ("F".into(), -2),
        ("H".into(), -2),
        ("V".into(), -2),
        ("W".into(), -2),
        ("Y".into(), -2),
        ("K".into(), -2),
        ("J".into(), -3),
        ("X".into(), -3),
        ("Q".into(), -4),
        ("Z".into(), -4),
        ("?".into(), -2),
    ])
}

fn tileset_symbol_for_kind(tileset: &Tileset, kind_id: &str) -> String {
    tileset
        .tile_kinds
        .iter()
        .find(|tk| tk.id == kind_id)
        .map(|tk| tk.symbol.clone())
        .unwrap_or_else(|| kind_id.to_string())
}

fn leftover_counts_from_rack(
    rack: &[String],
    placements: &[(CellId, Tile)],
) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for kid in rack {
        *counts.entry(kid.clone()).or_default() += 1;
    }
    for (_, tile) in placements {
        if let Some(entry) = counts.get_mut(&tile.kind_id)
            && *entry > 0
        {
            *entry -= 1;
        }
    }
    counts.retain(|_, v| *v > 0);
    counts
}

fn rack_leave_score(
    config: &AiConfig,
    tileset: &Tileset,
    leftover: &HashMap<String, usize>,
) -> i32 {
    leftover
        .iter()
        .map(|(kid, count)| {
            let sym = tileset_symbol_for_kind(tileset, kid).to_uppercase();
            let val = config
                .rack_leave
                .get(&sym)
                .or_else(|| config.rack_leave.get(kid))
                .copied()
                .unwrap_or(0);
            val * (*count as i32)
        })
        .sum()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedMove {
    pub candidate: CandidateMove,
    pub rack_leave: i32,
    pub board_equity: i32,
    pub endgame_penalty: i32,
    pub total: i32,
}

fn board_equity_bonus(state: &GameState, candidate: &CandidateMove) -> i32 {
    let mut bonus = 0;
    let placed: HashSet<CellId> = candidate.placements.iter().map(|(cid, _)| *cid).collect();
    for (cid, _) in &candidate.placements {
        for neigh in state.board.geom.neighbors(*cid) {
            if placed.contains(&neigh) {
                continue;
            }
            if state.board.cells[neigh.0 as usize].stack.is_empty() {
                bonus += 1;
            }
        }
    }
    bonus
}

fn endgame_penalty(state: &GameState, leftover: &HashMap<String, usize>) -> i32 {
    if state.bag.remaining() > 0 {
        return 0;
    }
    let mut penalty = 0;
    for (kind_id, count) in leftover {
        if *count == 0 {
            continue;
        }
        if let Some(kind) = state.tileset.tile_kinds.iter().find(|tk| tk.id == *kind_id) {
            penalty -= (*count as i32) * (kind.score as i32);
        }
    }
    penalty
}

pub fn evaluate_candidate_move(
    state: &GameState,
    candidate: CandidateMove,
    rack: &[String],
    config: &AiConfig,
) -> EvaluatedMove {
    let leftover = leftover_counts_from_rack(rack, &candidate.placements);
    let leave_score = rack_leave_score(config, &state.tileset, &leftover);
    let board_eq = board_equity_bonus(state, &candidate);
    let end_pen = endgame_penalty(state, &leftover);
    let total = candidate.score + leave_score + board_eq + end_pen;
    EvaluatedMove {
        candidate,
        rack_leave: leave_score,
        board_equity: board_eq,
        endgame_penalty: end_pen,
        total,
    }
}

pub fn best_move_greedy(
    state: &GameState,
    rules: &impl Rules,
    config: &AiConfig,
) -> Option<EvaluatedMove> {
    let mut ctx = SearchContext::new(config);
    best_move_inner(state, rules, config.lookahead_depth, &mut ctx)
}

pub fn best_move(
    state: &GameState,
    rules: &impl Rules,
    level: AiDifficulty,
) -> Option<EvaluatedMove> {
    let cfg = AiConfig::for_difficulty(level);
    best_move_greedy(state, rules, &cfg)
}

#[derive(Debug)]
struct BestCandidate {
    eval: EvaluatedMove,
    adjusted_total: i32,
}

struct SearchContext<'a> {
    config: &'a AiConfig,
    rng: Option<StdRng>,
    nodes: usize,
    deadline: Option<Instant>,
}

impl<'a> SearchContext<'a> {
    fn new(config: &'a AiConfig) -> Self {
        let mut rng = config.randomness.map(StdRng::seed_from_u64);
        if rng.is_none() && config.requires_rng() {
            rng = Some(StdRng::from_entropy());
        }
        Self {
            config,
            rng,
            nodes: 0,
            deadline: config
                .max_duration
                .map(|d| Instant::now().checked_add(d).unwrap_or(Instant::now())),
        }
    }

    fn rng_mut(&mut self) -> Option<&mut StdRng> {
        if self.rng.is_none() && (self.config.randomness.is_some() || self.config.requires_rng()) {
            self.rng = Some(match self.config.randomness {
                Some(seed) => StdRng::seed_from_u64(seed),
                None => StdRng::from_entropy(),
            });
        }
        self.rng.as_mut()
    }

    fn record_node(&mut self) {
        self.nodes = self.nodes.saturating_add(1);
    }

    fn node_limit_hit(&self) -> bool {
        self.config
            .max_nodes
            .map(|limit| self.nodes >= limit)
            .unwrap_or(false)
    }

    fn time_limit_hit(&self) -> bool {
        self.deadline
            .map(|deadline| Instant::now() >= deadline)
            .unwrap_or(false)
    }

    fn sample_noise(&mut self) -> i32 {
        let range = self.config.noise_range;
        if range <= 0 {
            return 0;
        }
        self.rng_mut()
            .map(|rng| rng.gen_range(-range..=range))
            .unwrap_or(0)
    }

    fn random_bool(&mut self, p: f64) -> bool {
        if p <= 0.0 {
            return false;
        }
        let p = p.min(1.0);
        self.rng_mut().map(|rng| rng.gen_bool(p)).unwrap_or(false)
    }
}

fn best_move_inner(
    state: &GameState,
    rules: &impl Rules,
    depth: usize,
    ctx: &mut SearchContext<'_>,
) -> Option<EvaluatedMove> {
    let pid = state.to_move.0;
    let rack: Vec<String> = state.players[pid]
        .rack
        .tiles
        .iter()
        .map(|t| t.kind_id.clone())
        .collect();
    let mut candidates = generate_moves(state, rules, &rack, ctx.config.max_move_len);
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by(|a, b| b.score.cmp(&a.score));

    let is_root = depth == ctx.config.lookahead_depth;
    if is_root {
        if let Some(limit) = ctx.config.candidate_limit
            && candidates.len() > limit
        {
            candidates.truncate(limit);
        }
    } else if ctx.config.reply_move_limit != usize::MAX
        && candidates.len() > ctx.config.reply_move_limit
    {
        candidates.truncate(ctx.config.reply_move_limit);
    }

    let mut best: Option<BestCandidate> = None;

    #[cfg(feature = "parallel")]
    if ctx.config.parallel_eval
        && depth == ctx.config.lookahead_depth
        && ctx.config.lookahead_depth == 0
        && ctx.config.noise_range == 0
        && ctx.config.randomness.is_none()
        && ctx.config.max_nodes.is_none()
        && ctx.config.max_duration.is_none()
    {
        let evals: Vec<(CandidateMove, EvaluatedMove)> = candidates
            .par_iter()
            .map(|cand| {
                let eval = evaluate_candidate_move(state, cand.clone(), &rack, ctx.config);
                (cand.clone(), eval)
            })
            .collect();
        for (_cand, eval) in evals {
            ctx.record_node();
            let adjusted = eval.total;
            match &mut best {
                None => {
                    best = Some(BestCandidate {
                        eval,
                        adjusted_total: adjusted,
                    });
                }
                Some(current) => {
                    let better = adjusted > current.adjusted_total
                        || (adjusted == current.adjusted_total && eval.total > current.eval.total)
                        || (adjusted == current.adjusted_total
                            && eval.total == current.eval.total
                            && eval.rack_leave > current.eval.rack_leave);
                    if better {
                        *current = BestCandidate {
                            eval,
                            adjusted_total: adjusted,
                        };
                    }
                }
            }
        }
        return best.map(|b| b.eval);
    }
    for cand in candidates {
        if ctx.node_limit_hit() && best.is_some() {
            break;
        }
        if ctx.time_limit_hit() && best.is_some() {
            break;
        }
        ctx.record_node();
        let mut eval = evaluate_candidate_move(state, cand.clone(), &rack, ctx.config);
        if depth > 0 && !ctx.node_limit_hit() && !ctx.time_limit_hit() {
            let draft = MoveDraft {
                placements: cand.placements.clone(),
            };
            if let Ok(validated) = rules.validate(state, &draft) {
                let score = rules.score(state, &validated);
                let mut next_state = state.clone();
                if rules.commit(&mut next_state, validated, &score).is_ok()
                    && let Some(reply) =
                        best_move_inner(&next_state, rules, depth.saturating_sub(1), ctx)
                {
                    eval.total -= reply.total;
                }
            }
        }
        let adjusted = eval.total + ctx.sample_noise();
        match &mut best {
            None => {
                best = Some(BestCandidate {
                    eval,
                    adjusted_total: adjusted,
                })
            }
            Some(current) => {
                let better = adjusted > current.adjusted_total
                    || (adjusted == current.adjusted_total && eval.total > current.eval.total)
                    || (adjusted == current.adjusted_total
                        && eval.total == current.eval.total
                        && eval.rack_leave > current.eval.rack_leave)
                    || (adjusted == current.adjusted_total
                        && eval.total == current.eval.total
                        && eval.rack_leave == current.eval.rack_leave
                        && ctx.random_bool(0.5));
                if better {
                    *current = BestCandidate {
                        eval,
                        adjusted_total: adjusted,
                    };
                }
            }
        }
        if ctx.time_limit_hit() && best.is_some() {
            break;
        }
    }
    best.map(|b| b.eval)
}

/// Generate naive horizontal moves at anchors (place to the right only).
/// Rack is a multiset of `kind_id` strings available to play.
pub fn generate_moves(
    state: &GameState,
    rules: &impl Rules,
    rack: &[String],
    max_len: usize,
) -> Vec<CandidateMove> {
    // find anchors: empty cells adjacent to any existing tile; if board empty, use center
    let mut anchors: Vec<CellId> = Vec::new();
    let any_on_board = state.board.cells.iter().any(|c| !c.stack.is_empty());
    if !any_on_board {
        anchors.push(CrosswordRules::center_cell(&state.board.geom));
    } else {
        for (idx, cell) in state.board.cells.iter().enumerate() {
            if !cell.stack.is_empty() {
                continue;
            }
            let id = CellId(idx as u32);
            if CrosswordRules::adjacent_to_existing(&state.board, &[id]) {
                anchors.push(id);
            }
        }
    }

    fn get_symbol<'a>(tileset: &'a Tileset, kind_id: &str) -> Option<(&'a TileKind, &'a str)> {
        for tk in &tileset.tile_kinds {
            if tk.id == kind_id {
                return Some((tk, tk.symbol.as_str()));
            }
        }
        None
    }
    fn find_kind_for_symbol<'a>(tileset: &'a Tileset, sym: &str) -> Option<&'a TileKind> {
        tileset
            .tile_kinds
            .iter()
            .find(|tk| !tk.is_blank && tk.symbol == sym)
    }

    // build helper to read cell including overlay
    #[derive(Default, Clone)]
    struct Overlay(std::collections::HashMap<CellId, Tile>);
    impl Overlay {
        fn get<'a>(&'a self, id: CellId, state: &'a GameState) -> Option<&'a Tile> {
            if let Some(t) = self.0.get(&id) {
                return Some(t);
            }
            let cell = &state.board.cells[id.0 as usize];
            cell.stack.last()
        }
    }

    fn perp_word(state: &GameState, ov: &Overlay, at: CellId, dir: (i32, i32)) -> String {
        let c0 = state.board.geom.from_cell_id(at).unwrap();
        let (dx, dy) = dir;
        // move negative direction
        let mut c = c0;
        loop {
            let prev = Coord2D {
                x: c.x - dx,
                y: c.y - dy,
            };
            if let Some(id) = state.board.geom.to_cell_id(prev)
                && ov.get(id, state).is_some()
            {
                c = prev;
                continue;
            }
            break;
        }
        let mut s = String::new();
        loop {
            if let Some(id) = state.board.geom.to_cell_id(c)
                && let Some(tile) = ov.get(id, state)
            {
                let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, tile);
                s.push_str(&sym);
                c = Coord2D {
                    x: c.x + dx,
                    y: c.y + dy,
                };
                continue;
            }
            break;
        }
        s
    }

    // Precompute cross-check sets (only when dictionary enforced)
    use std::collections::{HashMap as Map, HashSet as Set};
    fn gather_line(state: &GameState, start: Coord2D, step: (i32, i32)) -> (String, usize) {
        let mut s = String::new();
        let mut tiles = 0usize;
        let mut c = start;
        loop {
            if let Some(id) = state.board.geom.to_cell_id(c) {
                let cell = &state.board.cells[id.0 as usize];
                if let Some(t) = cell.stack.last() {
                    let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, t);
                    s.push_str(&sym);
                    tiles += 1;
                    c = Coord2D {
                        x: c.x + step.0,
                        y: c.y + step.1,
                    };
                    continue;
                }
            }
            break;
        }
        (s, tiles)
    }
    fn cross_checks(state: &GameState, dir: (i32, i32)) -> Map<CellId, Set<String>> {
        let mut out: Map<CellId, Set<String>> = Map::new();
        for (idx, cell) in state.board.cells.iter().enumerate() {
            if !cell.stack.is_empty() {
                continue;
            }
            let id = CellId(idx as u32);
            let c = state.board.geom.from_cell_id(id).unwrap();
            let above = Coord2D {
                x: c.x - dir.0,
                y: c.y - dir.1,
            };
            let below = Coord2D {
                x: c.x + dir.0,
                y: c.y + dir.1,
            };
            let (top, top_tiles) = gather_line(state, above, (-dir.0, -dir.1));
            let (bot, bot_tiles) = gather_line(state, below, (dir.0, dir.1));
            let base_tiles = top_tiles + bot_tiles;
            if base_tiles == 0 {
                continue;
            }
            let mut set: Set<String> = Set::new();
            // If a dictionary is attached, only include letters that produce a valid perpendicular word (length>1)
            if let Some(dict) = &state.dictionary {
                for tk in &state.tileset.tile_kinds {
                    if tk.is_blank {
                        continue;
                    }
                    let sym = &tk.symbol;
                    let w = format!("{}{}{}", top, sym, bot);
                    // Require at least 2 tiles in the perpendicular word
                    if base_tiles + 1 > 1 && dict.contains(&w) {
                        set.insert(sym.clone());
                    }
                }
            } else {
                for tk in &state.tileset.tile_kinds {
                    if !tk.is_blank {
                        set.insert(tk.symbol.clone());
                    }
                }
            }
            out.insert(id, set);
        }
        out
    }
    // compute vertical cross checks for horizontal plays, and horizontal for vertical plays
    let xchecks_vert = cross_checks(state, (0, 1));
    let xchecks_horz = cross_checks(state, (1, 0));

    // DFS to the right from anchor only (simplified); ensure anchor included
    #[allow(
        clippy::too_many_arguments,
        clippy::collapsible_if,
        clippy::manual_retain
    )]
    fn dfs_right(
        state: &GameState,
        rules: &impl Rules,
        anchor: CellId,
        rack: &mut std::collections::HashMap<String, usize>,
        built: String,
        pos: Coord2D,
        used: &mut Vec<(CellId, Tile)>,
        out: &mut Vec<CandidateMove>,
        max_len: usize,
        dir: (i32, i32),
        pdir: (i32, i32),
        mut gaddag: Option<(&GaddagDictionary, usize)>,
        xchecks: &Map<CellId, Set<String>>,
        blank_kinds: &[String],
    ) {
        let len_tokens = if let Some((gd, _)) = &gaddag {
            gd.tokenizer().segment(&built).len()
        } else {
            built.graphemes(true).count()
        };
        if len_tokens >= max_len {
            return;
        }
        // if cell has fixed tile, append and continue
        if let Some(id) = state.board.geom.to_cell_id(pos) {
            let cell = &state.board.cells[id.0 as usize];
            if let Some(t) = cell.stack.last() {
                let mut nb = built.clone();
                let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, t);
                nb.push_str(&sym);
                // GADDAG step for multi-char symbols or prefix prune
                if let Some((gd, node)) = gaddag {
                    if let Some(n2) = gd.step_symbol(node, &sym) {
                        gaddag = Some((gd, n2));
                    } else {
                        return;
                    }
                } else if let Some(dict) = &state.dictionary {
                    if !dict.has_prefix(&nb) {
                        return;
                    }
                }
                let next = Coord2D {
                    x: pos.x + dir.0,
                    y: pos.y + dir.1,
                };
                dfs_right(
                    state,
                    rules,
                    anchor,
                    rack,
                    nb,
                    next,
                    used,
                    out,
                    max_len,
                    dir,
                    pdir,
                    gaddag,
                    xchecks,
                    blank_kinds,
                );
                return;
            }
        } else {
            return;
        }

        // Try placing from rack: first, optional blank placements for letters not available as normal tiles
        // Compute candidate symbols set
        let id = state.board.geom.to_cell_id(pos).unwrap();
        if state.board.cells[id.0 as usize].stack.is_empty() {
            let mut cand_syms: Set<String> = Set::new();
            // base from cross-checks or all symbols
            if let Some(set) = xchecks.get(&id) {
                for s in set {
                    cand_syms.insert(s.clone());
                }
            } else {
                // derive from dictionary prefixes if available; else from tileset symbols
                if let Some(dict) = &state.dictionary {
                    for tk in &state.tileset.tile_kinds {
                        if tk.is_blank {
                            continue;
                        }
                        if dict.has_prefix(&tk.symbol) {
                            cand_syms.insert(tk.symbol.clone());
                        }
                    }
                } else {
                    for tk in &state.tileset.tile_kinds {
                        if !tk.is_blank {
                            cand_syms.insert(tk.symbol.clone());
                        }
                    }
                }
            }
            // filter by gaddag child arcs if available
            if let Some((gd, node)) = gaddag {
                cand_syms = cand_syms
                    .into_iter()
                    .filter(|s| gd.step_symbol(node, s).is_some())
                    .collect();
            }
            // For each symbol, if no normal tile available in rack, but a blank exists, place blank
            // Augment candidate symbols for blanks with A..Z dictionary prefixes
            let mut blank_syms = cand_syms.clone();
            if let Some(dict) = &state.dictionary {
                for ch in 'A'..='Z' {
                    let s = ch.to_string();
                    if dict.has_prefix(&s) {
                        blank_syms.insert(s);
                    }
                }
            }
            for sym in blank_syms.into_iter() {
                // normal tile kind for symbol
                let normal_kind = find_kind_for_symbol(&state.tileset, &sym).map(|k| k.id.clone());
                let normal_avail = normal_kind
                    .as_ref()
                    .and_then(|kid| rack.get(kid))
                    .copied()
                    .unwrap_or(0)
                    > 0;
                if normal_avail {
                    continue;
                }
                // find a blank
                let mut chosen_blank: Option<String> = None;
                for bk in blank_kinds {
                    if rack.get(bk).copied().unwrap_or(0) > 0 {
                        chosen_blank = Some(bk.clone());
                        break;
                    }
                }
                if let Some(bid) = chosen_blank {
                    // perpendicular check via overlay
                    let mut ov = Overlay::default();
                    for (cid, tile) in used.iter() {
                        ov.0.insert(*cid, tile.clone());
                    }
                    ov.0.insert(
                        id,
                        Tile {
                            kind_id: bid.clone(),
                            mark: Some(sym.clone()),
                        },
                    );
                    let vword = perp_word(state, &ov, id, pdir);
                    let vtiles = {
                        let c0 = id; // reuse helper logic inline to avoid borrow issues
                        let mut count = 0usize;
                        let mut c = state.board.geom.from_cell_id(c0).unwrap();
                        let (dx, dy) = pdir;
                        loop {
                            let prev = Coord2D {
                                x: c.x - dx,
                                y: c.y - dy,
                            };
                            if let Some(pid) = state.board.geom.to_cell_id(prev) {
                                if ov.get(pid, state).is_some() {
                                    c = prev;
                                    continue;
                                }
                            }
                            break;
                        }
                        loop {
                            if let Some(pid) = state.board.geom.to_cell_id(c) {
                                if ov.get(pid, state).is_some() {
                                    count += 1;
                                    c = Coord2D {
                                        x: c.x + dx,
                                        y: c.y + dy,
                                    };
                                    continue;
                                }
                            }
                            break;
                        }
                        count
                    };
                    if vtiles > 1 {
                        if let Some(dict) = &state.dictionary {
                            if !dict.contains(&vword) {
                                continue;
                            }
                        }
                    }
                    // Append symbol and recurse
                    let mut nb = built.clone();
                    nb.push_str(&sym);
                    // GADDAG step or prefix check
                    let mut next_gaddag = gaddag;
                    if let Some((gd, node)) = next_gaddag {
                        if let Some(n2) = gd.step_symbol(node, &sym) {
                            next_gaddag = Some((gd, n2));
                        } else {
                            continue;
                        }
                    } else if let Some(dict) = &state.dictionary {
                        if !dict.has_prefix(&nb) {
                            continue;
                        }
                    }
                    // consume blank
                    *rack.get_mut(&bid).unwrap() -= 1;
                    used.push((
                        id,
                        Tile {
                            kind_id: bid.clone(),
                            mark: Some(sym.clone()),
                        },
                    ));
                    let next = Coord2D {
                        x: pos.x + dir.0,
                        y: pos.y + dir.1,
                    };
                    dfs_right(
                        state,
                        rules,
                        anchor,
                        rack,
                        nb,
                        next,
                        used,
                        out,
                        max_len,
                        dir,
                        pdir,
                        next_gaddag,
                        xchecks,
                        blank_kinds,
                    );
                    used.pop();
                    *rack.get_mut(&bid).unwrap() += 1;
                }
            }
        }

        // Try placing from rack (normal tiles)
        for (kind_id, cnt) in rack.clone() {
            // iterate snapshot
            if cnt == 0 {
                continue;
            }
            // place here
            let id = state.board.geom.to_cell_id(pos).unwrap();
            // Only place if empty
            if !state.board.cells[id.0 as usize].stack.is_empty() {
                continue;
            }
            // Resolve symbol
            let Some((_tk, sym)) = get_symbol(&state.tileset, &kind_id) else {
                continue;
            };
            // Cross-check set pruning (if present for this cell)
            if let Some(set) = xchecks.get(&id) {
                if !set.contains(sym) {
                    continue;
                }
            }
            // Cross-check vertical
            let mut ov = Overlay::default();
            for (cid, tile) in used.iter() {
                ov.0.insert(*cid, tile.clone());
            }
            ov.0.insert(
                id,
                Tile {
                    kind_id: kind_id.clone(),
                    mark: None,
                },
            );
            let vword = perp_word(state, &ov, id, pdir);
            let vtiles = {
                let c0 = id;
                let mut count = 0usize;
                let mut c = state.board.geom.from_cell_id(c0).unwrap();
                let (dx, dy) = pdir;
                loop {
                    let prev = Coord2D {
                        x: c.x - dx,
                        y: c.y - dy,
                    };
                    if let Some(pid) = state.board.geom.to_cell_id(prev) {
                        if ov.get(pid, state).is_some() {
                            c = prev;
                            continue;
                        }
                    }
                    break;
                }
                loop {
                    if let Some(pid) = state.board.geom.to_cell_id(c) {
                        if ov.get(pid, state).is_some() {
                            count += 1;
                            c = Coord2D {
                                x: c.x + dx,
                                y: c.y + dy,
                            };
                            continue;
                        }
                    }
                    break;
                }
                count
            };
            if vtiles > 1 {
                if let Some(dict) = &state.dictionary {
                    if !dict.contains(&vword) {
                        continue;
                    }
                }
            }
            // Append and recurse / also consider committing as a move end
            let mut nb = built.clone();
            nb.push_str(sym);
            // GADDAG step or prefix prune
            let mut next_gaddag = gaddag;
            if let Some((gd, node)) = next_gaddag {
                if let Some(n2) = gd.step_symbol(node, sym) {
                    next_gaddag = Some((gd, n2));
                } else {
                    continue;
                }
            } else if let Some(dict) = &state.dictionary {
                if !dict.has_prefix(&nb) {
                    continue;
                }
            }
            // Prepare used/rack
            *rack.get_mut(&kind_id).unwrap() -= 1;
            used.push((
                id,
                Tile {
                    kind_id: kind_id.clone(),
                    mark: None,
                },
            ));

            // Attempt to finalize this sequence as a play covering anchor
            // Build ValidatedMove directly (we already enforce line/contiguity/anchor here) and score
            if used.iter().any(|(cid, _)| *cid == anchor)
                || state.board.cells[anchor.0 as usize].stack.last().is_some()
            {
                let v = ValidatedMove {
                    placements: used.clone(),
                    line_is_row: dir.1 == 0,
                };
                let sc = rules.score(state, &v);
                if sc.total >= 0 {
                    // valid dict words
                    out.push(CandidateMove {
                        placements: used.clone(),
                        word: sc.main_word.clone(),
                        score: sc.total,
                    });
                }
            }

            // Recurse to the right
            let next = Coord2D {
                x: pos.x + dir.0,
                y: pos.y + dir.1,
            };
            dfs_right(
                state,
                rules,
                anchor,
                rack,
                nb,
                next,
                used,
                out,
                max_len,
                dir,
                pdir,
                next_gaddag,
                xchecks,
                blank_kinds,
            );

            // backtrack
            used.pop();
            *rack.get_mut(&kind_id).unwrap() += 1;
        }
    }

    let mut out: Vec<CandidateMove> = Vec::new();
    #[allow(
        clippy::too_many_arguments,
        clippy::collapsible_if,
        clippy::collapsible_else_if,
        clippy::manual_retain
    )]
    fn dfs_left_then_right(
        state: &GameState,
        rules: &impl Rules,
        anchor: CellId,
        rack: &mut std::collections::HashMap<String, usize>,
        built: String,
        start: Coord2D,
        used: &mut Vec<(CellId, Tile)>,
        out: &mut Vec<CandidateMove>,
        max_len: usize,
        dir: (i32, i32),
        pdir: (i32, i32),
        gaddag_pre: Option<(&GaddagDictionary, usize)>,
        xchecks: &Map<CellId, Set<String>>,
        blank_kinds: &[String],
    ) {
        // First, try right expansion with current built, using GADDAG post-sep node if available
        let mut post_sep = None;
        if let Some((gd, node)) = gaddag_pre {
            if let Some(n2) = gd.step_token(node, gd.sep_token()) {
                post_sep = Some((gd, n2));
            }
        }
        dfs_right(
            state,
            rules,
            anchor,
            rack,
            built.clone(),
            start,
            used,
            out,
            max_len,
            dir,
            pdir,
            post_sep,
            xchecks,
            blank_kinds,
        );
        // Then, attempt to place one more letter to the left and recurse
        let left = Coord2D {
            x: start.x - dir.0,
            y: start.y - dir.1,
        };
        if let Some(left_id) = state.board.geom.to_cell_id(left) {
            // stop if left cell occupied by board tile
            if !state.board.cells[left_id.0 as usize].stack.is_empty() {
                return;
            }
            // try rack letters at left
            // Try blank placements at left (letters not available as normal)
            let mut cand_syms: Set<String> = Set::new();
            if let Some(set) = xchecks.get(&left_id) {
                for s in set {
                    cand_syms.insert(s.clone());
                }
            } else {
                if let Some(dict) = &state.dictionary {
                    for tk in &state.tileset.tile_kinds {
                        if !tk.is_blank && dict.has_prefix(&tk.symbol) {
                            cand_syms.insert(tk.symbol.clone());
                        }
                    }
                } else {
                    for tk in &state.tileset.tile_kinds {
                        if !tk.is_blank {
                            cand_syms.insert(tk.symbol.clone());
                        }
                    }
                }
            }
            if let Some((gd, base)) = gaddag_pre {
                cand_syms = cand_syms
                    .into_iter()
                    .filter(|s| gd.step_symbol(base, s).is_some())
                    .collect();
            }
            let mut blank_syms = cand_syms.clone();
            if let Some(dict) = &state.dictionary {
                for ch in 'A'..='Z' {
                    let s = ch.to_string();
                    if dict.has_prefix(&s) {
                        blank_syms.insert(s);
                    }
                }
            }
            for sym in blank_syms.into_iter() {
                let normal_kind = find_kind_for_symbol(&state.tileset, &sym).map(|k| k.id.clone());
                let normal_avail = normal_kind
                    .as_ref()
                    .and_then(|kid| rack.get(kid))
                    .copied()
                    .unwrap_or(0)
                    > 0;
                if normal_avail {
                    continue;
                }
                let mut chosen_blank: Option<String> = None;
                for bk in blank_kinds {
                    if rack.get(bk).copied().unwrap_or(0) > 0 {
                        chosen_blank = Some(bk.clone());
                        break;
                    }
                }
                if let Some(bid) = chosen_blank {
                    // cross-check perpendicular already satisfied by cand_syms/xchecks; still validate dict if longer
                    let mut nb = String::new();
                    nb.push_str(&sym);
                    nb.push_str(&built);
                    let mut next_g_pre2 = gaddag_pre;
                    if let Some((gd, base2)) = next_g_pre2 {
                        if let Some(n2) = gd.step_symbol(base2, &sym) {
                            next_g_pre2 = Some((gd, n2));
                        } else {
                            continue;
                        }
                    }
                    // consume blank and recurse
                    *rack.get_mut(&bid).unwrap() -= 1;
                    used.push((
                        left_id,
                        Tile {
                            kind_id: bid.clone(),
                            mark: Some(sym.clone()),
                        },
                    ));
                    dfs_left_then_right(
                        state,
                        rules,
                        anchor,
                        rack,
                        nb,
                        left,
                        used,
                        out,
                        max_len,
                        dir,
                        pdir,
                        next_g_pre2,
                        xchecks,
                        blank_kinds,
                    );
                    used.pop();
                    *rack.get_mut(&bid).unwrap() += 1;
                }
            }

            for (kind_id, cnt) in rack.clone() {
                if cnt == 0 {
                    continue;
                }
                // cross-check perpendicular at left position
                let Some((_tk, sym)) = get_symbol(&state.tileset, &kind_id) else {
                    continue;
                };

                let mut ov = Overlay::default();
                for (cid, tile) in used.iter() {
                    ov.0.insert(*cid, tile.clone());
                }
                ov.0.insert(
                    left_id,
                    Tile {
                        kind_id: kind_id.clone(),
                        mark: None,
                    },
                );
                let vword = perp_word(state, &ov, left_id, pdir);
                // count perpendicular tiles (not characters)
                let vtiles = {
                    let mut count = 0usize;
                    let mut c = state.board.geom.from_cell_id(left_id).unwrap();
                    let (dx, dy) = pdir;
                    loop {
                        let prev = Coord2D {
                            x: c.x - dx,
                            y: c.y - dy,
                        };
                        if let Some(pid) = state.board.geom.to_cell_id(prev) {
                            if ov.get(pid, state).is_some() {
                                c = prev;
                                continue;
                            }
                        }
                        break;
                    }
                    loop {
                        if let Some(pid) = state.board.geom.to_cell_id(c) {
                            if ov.get(pid, state).is_some() {
                                count += 1;
                                c = Coord2D {
                                    x: c.x + dx,
                                    y: c.y + dy,
                                };
                                continue;
                            }
                        }
                        break;
                    }
                    count
                };
                if vtiles > 1 {
                    if let Some(dict) = &state.dictionary {
                        if !dict.contains(&vword) {
                            continue;
                        }
                    }
                }
                // prepend symbol to built
                let mut nb = String::new();
                nb.push_str(sym);
                nb.push_str(&built);
                // If GADDAG available, step pre-sep with this char; else fallback to prefix check
                let mut next_g_pre = gaddag_pre;
                if let Some((gd, base)) = next_g_pre {
                    if let Some(n2) = gd.step_symbol(base, sym) {
                        next_g_pre = Some((gd, n2));
                    } else {
                        continue;
                    }
                } else if let Some(dict) = &state.dictionary {
                    if !dict.has_prefix(&nb) {
                        continue;
                    }
                }
                // place and recurse
                *rack.get_mut(&kind_id).unwrap() -= 1;
                used.push((
                    left_id,
                    Tile {
                        kind_id: kind_id.clone(),
                        mark: None,
                    },
                ));
                dfs_left_then_right(
                    state,
                    rules,
                    anchor,
                    rack,
                    nb,
                    left,
                    used,
                    out,
                    max_len,
                    dir,
                    pdir,
                    next_g_pre,
                    xchecks,
                    blank_kinds,
                );
                used.pop();
                *rack.get_mut(&kind_id).unwrap() += 1;
            }
        }
    }
    fn context_prefix(state: &GameState, pos: Coord2D, dir: (i32, i32)) -> String {
        // find beginning of contiguous run ending just before pos
        let mut c = pos;
        loop {
            let prev = Coord2D {
                x: c.x - dir.0,
                y: c.y - dir.1,
            };
            if let Some(id) = state.board.geom.to_cell_id(prev)
                && state.board.cells[id.0 as usize].stack.last().is_some()
            {
                c = prev;
                continue;
            }
            break;
        }
        // build until pos (excluding pos)
        let mut s = String::new();
        loop {
            if c.x == pos.x && c.y == pos.y {
                break;
            }
            if let Some(id) = state.board.geom.to_cell_id(c)
                && let Some(t) = state.board.cells[id.0 as usize].stack.last()
            {
                let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, t);
                s.push_str(&sym);
                c = Coord2D {
                    x: c.x + dir.0,
                    y: c.y + dir.1,
                };
                continue;
            }
            break;
        }
        s
    }

    // Try to obtain GADDAG dictionary reference for automaton-guided search
    let gaddag_pre_seed: Option<&GaddagDictionary> = state
        .dictionary
        .as_deref()
        .and_then(|d| d.as_any().downcast_ref::<GaddagDictionary>());

    // Identify blank kind IDs
    let blank_kinds: Vec<String> = state
        .tileset
        .tile_kinds
        .iter()
        .filter(|tk| tk.is_blank)
        .map(|tk| tk.id.clone())
        .collect();

    if state.board.geom.has_graph() {
        return generate_moves_graph_basic(state, rules, rack, max_len, &anchors);
    }

    for a in anchors {
        let start = state.board.geom.from_cell_id(a).unwrap();
        let mut rack_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for k in rack {
            *rack_counts.entry(k.clone()).or_default() += 1;
        }
        // horizontal with left context
        let h_prefix = context_prefix(state, start, (1, 0));
        // Compute initial GADDAG pre-sep node for left context
        let mut pre: Option<(&GaddagDictionary, usize)> = None;
        if let Some(gd) = gaddag_pre_seed
            && let Some(cur) = GaddagCursor::new(gd, &h_prefix)
        {
            pre = Some((gd, cur.pre_node()));
        }
        dfs_left_then_right(
            state,
            rules,
            a,
            &mut rack_counts.clone(),
            h_prefix,
            start,
            &mut Vec::new(),
            &mut out,
            max_len,
            (1, 0),
            (0, 1),
            pre,
            &xchecks_vert,
            &blank_kinds,
        );
        // vertical with up context
        let v_prefix = context_prefix(state, start, (0, 1));
        let mut pre_v: Option<(&GaddagDictionary, usize)> = None;
        if let Some(gd) = gaddag_pre_seed
            && let Some(cur) = GaddagCursor::new(gd, &v_prefix)
        {
            pre_v = Some((gd, cur.pre_node()));
        }
        dfs_left_then_right(
            state,
            rules,
            a,
            &mut rack_counts.clone(),
            v_prefix,
            start,
            &mut Vec::new(),
            &mut out,
            max_len,
            (0, 1),
            (1, 0),
            pre_v,
            &xchecks_horz,
            &blank_kinds,
        );
    }
    // de-duplicate identical placement sets (same cells and assigned symbols)
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();
    let mut dedup: Vec<CandidateMove> = Vec::new();
    for cm in out.into_iter() {
        let mut key_parts: Vec<String> = cm
            .placements
            .iter()
            .map(|(cid, t)| {
                format!(
                    "{}:{}:{}",
                    cid.0,
                    t.kind_id,
                    t.mark.clone().unwrap_or_default()
                )
            })
            .collect();
        key_parts.sort();
        let key = format!("{}|{}", cm.word, key_parts.join(","));
        if seen.insert(key) {
            dedup.push(cm);
        }
    }
    dedup.sort_by_key(|cm| (-cm.score, cm.word.clone(), cm.placements.len()));
    dedup
}

fn generate_moves_graph_basic(
    state: &GameState,
    rules: &impl Rules,
    rack: &[String],
    max_len: usize,
    anchors: &[CellId],
) -> Vec<CandidateMove> {
    use std::collections::{HashMap, HashSet};

    fn build_line(geom: &RectGridGeometry, start: CellId, tag: &str) -> Vec<CellId> {
        use std::collections::{HashSet, VecDeque};
        let mut visited: HashSet<CellId> = HashSet::new();
        let mut deque: VecDeque<CellId> = VecDeque::new();
        visited.insert(start);
        deque.push_back(start);
        for pass in 0..2 {
            let mut current = start;
            loop {
                let mut next_opt = None;
                for (n, t) in geom.neighbors_with_tags(current) {
                    if t == tag && !visited.contains(&n) {
                        next_opt = Some(n);
                        break;
                    }
                }
                if let Some(next_id) = next_opt {
                    if pass == 0 {
                        deque.push_back(next_id);
                    } else {
                        deque.push_front(next_id);
                    }
                    visited.insert(next_id);
                    current = next_id;
                } else {
                    break;
                }
            }
        }
        deque.into_iter().collect()
    }

    struct ExploreCtx<'a> {
        state: &'a GameState,
        rules: &'a dyn Rules,
        tile_symbols: &'a [(String, String)],
        blank_ids: &'a [String],
        seen: &'a mut HashSet<String>,
        out: &'a mut Vec<CandidateMove>,
    }

    fn explore_segment(
        segment: &[CellId],
        idx: usize,
        rack_counts: &mut HashMap<String, usize>,
        placements: &mut Vec<(CellId, Tile)>,
        ctx: &mut ExploreCtx<'_>,
    ) {
        if idx == segment.len() {
            if placements.is_empty() {
                return;
            }
            let draft = MoveDraft {
                placements: placements.clone(),
            };
            if let Ok(validated) = ctx.rules.validate(ctx.state, &draft) {
                let sc = ctx.rules.score(ctx.state, &validated);
                if sc.total >= 0 {
                    let mut key_parts: Vec<String> = placements
                        .iter()
                        .map(|(cid, tile)| {
                            format!(
                                "{}:{}:{}",
                                cid.0,
                                tile.kind_id,
                                tile.mark.clone().unwrap_or_default()
                            )
                        })
                        .collect();
                    key_parts.sort();
                    let key = format!("{}|{}", key_parts.join(";"), sc.main_word);
                    if ctx.seen.insert(key) {
                        ctx.out.push(CandidateMove {
                            placements: placements.clone(),
                            word: sc.main_word,
                            score: sc.total,
                        });
                    }
                }
            }
            return;
        }

        let cid = segment[idx];
        let cell = &ctx.state.board.cells[cid.0 as usize];
        if let Some(tile) = cell.stack.last() {
            let (_, _sym) = CrosswordRules::tile_symbol_and_score(&ctx.state.tileset, tile);
            explore_segment(segment, idx + 1, rack_counts, placements, ctx);
            return;
        }

        // Try normal tiles
        for (kind_id, _symbol) in ctx.tile_symbols.iter() {
            let available = rack_counts.get(kind_id).copied().unwrap_or(0);
            if available == 0 {
                continue;
            }
            {
                let entry = rack_counts.get_mut(kind_id).unwrap();
                *entry -= 1;
            }
            placements.push((
                cid,
                Tile {
                    kind_id: kind_id.clone(),
                    mark: None,
                },
            ));
            explore_segment(segment, idx + 1, rack_counts, placements, ctx);
            placements.pop();
            {
                let entry = rack_counts.get_mut(kind_id).unwrap();
                *entry += 1;
            }
        }

        // Try blank tiles
        for blank_id in ctx.blank_ids {
            let available = rack_counts.get(blank_id).copied().unwrap_or(0);
            if available == 0 {
                continue;
            }
            {
                let entry = rack_counts.get_mut(blank_id).unwrap();
                *entry -= 1;
            }
            for (_, symbol) in ctx.tile_symbols.iter() {
                placements.push((
                    cid,
                    Tile {
                        kind_id: blank_id.clone(),
                        mark: Some(symbol.clone()),
                    },
                ));
                explore_segment(segment, idx + 1, rack_counts, placements, ctx);
                placements.pop();
            }
            {
                let entry = rack_counts.get_mut(blank_id).unwrap();
                *entry += 1;
            }
        }
    }

    let tile_symbols: Vec<(String, String)> = state
        .tileset
        .tile_kinds
        .iter()
        .filter(|tk| !tk.is_blank)
        .map(|tk| (tk.id.clone(), tk.symbol.clone()))
        .collect();
    let blank_ids: Vec<String> = state
        .tileset
        .tile_kinds
        .iter()
        .filter(|tk| tk.is_blank)
        .map(|tk| tk.id.clone())
        .collect();

    let rack_template: HashMap<String, usize> = {
        let mut map = HashMap::new();
        for k in rack {
            *map.entry(k.clone()).or_default() += 1;
        }
        map
    };

    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut ctx = ExploreCtx {
        state,
        rules: rules as &dyn Rules,
        tile_symbols: &tile_symbols,
        blank_ids: &blank_ids,
        seen: &mut seen,
        out: &mut out,
    };

    for &anchor in anchors {
        if !state.board.cells[anchor.0 as usize].stack.is_empty() {
            continue;
        }
        let mut tags: HashSet<String> = HashSet::new();
        for (_, tag) in state.board.geom.neighbors_with_tags(anchor) {
            tags.insert(tag.to_string());
        }
        if tags.is_empty() {
            continue;
        }

        for tag in tags {
            let line = build_line(&state.board.geom, anchor, &tag);
            if line.is_empty() {
                continue;
            }
            let Some(anchor_idx) = line.iter().position(|id| *id == anchor) else {
                continue;
            };
            let line_len = line.len();
            for start in 0..=anchor_idx {
                for end in anchor_idx..line_len {
                    let seg_len = end - start + 1;
                    if seg_len == 0 || seg_len > max_len {
                        continue;
                    }
                    let segment = &line[start..=end];
                    let blanks_needed = segment
                        .iter()
                        .filter(|cid| state.board.cells[cid.0 as usize].stack.is_empty())
                        .count();
                    if blanks_needed == 0 {
                        continue;
                    }
                    if blanks_needed > rack.len() {
                        continue;
                    }

                    let mut rack_counts = rack_template.clone();
                    let mut placements: Vec<(CellId, Tile)> = Vec::new();
                    explore_segment(segment, 0, &mut rack_counts, &mut placements, &mut ctx);
                }
            }
        }
    }

    out
}

impl Player {
    fn rack_size(&self) -> Option<usize> {
        Some(self.rack_capacity.max(1))
    }
}

// -------- Dictionary Engine (Phase 3) --------
// Moved to module `dict`; re-exported here for compatibility
pub mod dict;
pub use dict::{Dictionary, DictionaryOptions, SetDictionary, FstDictionary, DawgDictionary, GaddagDictionary, GaddagCursor, GaddagRight};

/* old inlined trait moved to dict */
// (Dictionary trait moved to dict)

// (SetDictionary moved to dict)

// (FstDictionary moved to dict)

/* moved: impl FstDictionary {
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut v: Vec<String> = iter
            .into_iter()
            .map(|w| {
                let mut s = nfc(w.into());
                if case_fold {
                    s = s.to_lowercase();
                }
                s
            })
            .collect();
        v.sort();
        v.dedup();
        let set = fst::Set::from_iter(v.iter()).expect("build fst set");
        Self {
            set,
            case_fold,
            norm: NormalizationMode::NFC,
        }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
        opts: DictionaryOptions,
    ) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut v: Vec<String> = Vec::new();
        for line in reader.lines() {
            let s = line?;
            let s = s.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let mut w = normalize_with_mode(s, opts.norm);
            if opts.case_fold {
                w = w.to_lowercase();
            }
            let len = opts.tokenizer.segment(&w).len();
            if let Some(min) = opts.min_len
                && len < min
            {
                continue;
            }
            if let Some(max) = opts.max_len
                && len > max
            {
                continue;
            }
            v.push(w);
        }
        v.sort();
        v.dedup();
        let set = fst::Set::from_iter(v.iter()).expect("build fst set");
        Ok(Self {
            set,
            case_fold: opts.case_fold,
            norm: opts.norm,
        })
    }
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D, case_fold: bool) -> Result<Self, fst::Error> {
        let set = fst::Set::new(bytes.as_ref().to_vec())?;
        Ok(Self {
            set,
            case_fold,
            norm: NormalizationMode::NFC,
        })
    }

    pub fn from_words_opts<I, S>(iter: I, opts: DictionaryOptions) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut v: Vec<String> = iter
            .into_iter()
            .map(|w| {
                let mut s = normalize_with_mode(w.into(), opts.norm);
                if opts.case_fold {
                    s = s.to_lowercase();
                }
                s
            })
            .collect();
        v.sort();
        v.dedup();
        let set = fst::Set::from_iter(v.iter()).expect("build fst set");
        Self {
            set,
            case_fold: opts.case_fold,
            norm: opts.norm,
        }
    }
}

impl Dictionary for FstDictionary {
    fn contains(&self, word: &str) -> bool {
        let mut s = normalize_with_mode(word, self.norm);
        if self.case_fold {
            s = s.to_lowercase();
        }
        self.set.contains(&s)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        use fst::{IntoStreamer, automaton::Str};
        let mut p = normalize_with_mode(prefix, self.norm);
        if self.case_fold {
            p = p.to_lowercase();
        }
        let aut = Str::new(&p).starts_with();
        let mut stream = self.set.search(aut).into_stream();
        stream.next().is_some()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn boxed_clone(&self) -> Box<dyn Dictionary + Send + Sync> {
        Box::new(self.clone())
    }
}
*/

// Simple DAWG/Trie implementation with prefix search (Phase 11)
/* moved: #[derive(Debug, Clone, Default)]
pub struct DawgDictionary {
    nodes: Vec<DawgNode>,
    case_fold: bool,
    norm: NormalizationMode,
    tokenizer: TokenizerRef,
}

#[derive(Debug, Clone, Default)]
struct DawgNode {
    edges: std::collections::HashMap<String, usize>,
    terminal: bool,
}

impl DawgDictionary {
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let opts = DictionaryOptions {
            case_fold,
            ..Default::default()
        };
        Self::from_words_opts(iter, opts)
    }

    pub fn from_words_opts<I, S>(iter: I, opts: DictionaryOptions) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut words: Vec<String> = iter
            .into_iter()
            .map(|w| {
                let mut s = normalize_with_mode(w.into(), opts.norm);
                if opts.case_fold {
                    s = s.to_lowercase();
                }
                s
            })
            .collect();
        words.sort();
        words.dedup();
        Self::build_from_words(words, opts)
    }

    fn build_from_words(words: Vec<String>, opts: DictionaryOptions) -> Self {
        let DictionaryOptions {
            case_fold,
            min_len: _,
            max_len: _,
            norm,
            tokenizer,
        } = opts;
        let mut nodes = vec![DawgNode::default()];
        for w in &words {
            let tokens = tokenizer.segment(w);
            if tokens.is_empty() {
                continue;
            }
            let mut node = 0usize;
            for token in tokens {
                let next = if let Some(&id) = nodes[node].edges.get(&token) {
                    id
                } else {
                    let id = nodes.len();
                    nodes.push(DawgNode::default());
                    nodes[node].edges.insert(token.clone(), id);
                    id
                };
                node = next;
            }
            nodes[node].terminal = true;
        }
        Self {
            nodes,
            case_fold,
            norm,
            tokenizer,
        }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
        opts: DictionaryOptions,
    ) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut v: Vec<String> = Vec::new();
        for line in reader.lines() {
            let s = line?;
            let s = s.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let mut w = normalize_with_mode(s, opts.norm);
            if opts.case_fold {
                w = w.to_lowercase();
            }
            let len = opts.tokenizer.segment(&w).len();
            if let Some(min) = opts.min_len
                && len < min
            {
                continue;
            }
            if let Some(max) = opts.max_len
                && len > max
            {
                continue;
            }
            v.push(w);
        }
        Ok(Self::from_words_opts(v, opts))
    }

    pub fn tokenizer(&self) -> &TokenizerRef {
        &self.tokenizer
    }
}

impl Dictionary for DawgDictionary {
    fn contains(&self, word: &str) -> bool {
        let mut s = normalize_with_mode(word, self.norm);
        if self.case_fold {
            s = s.to_lowercase();
        }
        let mut node = 0usize;
        for token in self.tokenizer.segment(&s) {
            if let Some(&nxt) = self.nodes[node].edges.get(&token) {
                node = nxt;
            } else {
                return false;
            }
        }
        self.nodes.get(node).map(|n| n.terminal).unwrap_or(false)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        let mut p = normalize_with_mode(prefix, self.norm);
        if self.case_fold {
            p = p.to_lowercase();
        }
        let mut node = 0usize;
        for token in self.tokenizer.segment(&p) {
            if let Some(&nxt) = self.nodes[node].edges.get(&token) {
                node = nxt;
            } else {
                return false;
            }
        }
        true
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn boxed_clone(&self) -> Box<dyn Dictionary + Send + Sync> {
        Box::new(self.clone())
    }
}
*/

/* moved: #[derive(Debug, Clone)]
pub struct GaddagDictionary {
    forward: FstDictionary,
    // packed graph
    g_nodes: Vec<PackedNode>,
    g_arcs: Vec<PackedArc>,
    // symbol mapping
    sym2id: std::collections::HashMap<String, u16>,
    id2sym: Vec<String>,
    sep: String,
    sep_id: u16,
    tokenizer: TokenizerRef,
}

#[derive(Debug, Clone, Copy)]
struct PackedNode {
    offset: u32,
    degree: u16,
    flags: u16, // bit 0 => terminal
}
impl PackedNode {
    #[inline]
    fn terminal(&self) -> bool {
        (self.flags & 1) != 0
    }
}

#[derive(Debug, Clone, Copy)]
struct PackedArc {
    label: u16,
    target: u32,
}

// Transient builder node (u16-labeled sorted map)
#[derive(Default)]
struct BuildNode {
    edges: std::collections::BTreeMap<u16, usize>,
    terminal: bool,
}

// On-disk representation for packed GADDAG
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackedNodeDisk {
    offset: u32,
    degree: u16,
    flags: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackedArcDisk {
    label: u16,
    target: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GaddagDiskImage {
    sep: String,
    sep_id: u16,
    id2sym: Vec<String>,
    nodes: Vec<PackedNodeDisk>,
    arcs: Vec<PackedArcDisk>,
    words: Vec<String>,
    case_fold: bool,
    // 0 = NFC, 1 = NFKC
    norm_mode: u8,
}

impl GaddagDictionary {
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let opts = DictionaryOptions {
            case_fold,
            ..Default::default()
        };
        Self::from_words_opts(iter, opts)
    }

    pub fn from_words_opts<I, S>(iter: I, opts: DictionaryOptions) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let DictionaryOptions { case_fold, min_len, max_len, norm, tokenizer } = opts;
        let sep = "+".to_string();

        // Normalize/filter words first
        let mut words: Vec<String> = Vec::new();
        for w in iter.into_iter() {
            let mut s = normalize_with_mode(w.into(), norm);
            if case_fold { s = s.to_lowercase(); }
            if s.is_empty() { continue; }
            let tks = tokenizer.segment(&s);
            if tks.is_empty() { continue; }
            let len = tks.len();
            if let Some(min) = min_len { if len < min { continue; } }
            if let Some(max) = max_len { if len > max { continue; } }
            words.push(s);
        }

        // Build small symbol table
        let mut sym2id: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
        let mut id2sym: Vec<String> = Vec::new();
        let mut intern = |sym: &str,
                          map: &mut std::collections::HashMap<String, u16>,
                          vec: &mut Vec<String>| -> u16 {
            if let Some(&id) = map.get(sym) { return id; }
            let id = vec.len() as u16;
            map.insert(sym.to_string(), id);
            vec.push(sym.to_string());
            id
        };
        for s in &words {
            for tk in tokenizer.segment(s) {
                let _ = intern(&tk, &mut sym2id, &mut id2sym);
            }
        }
        let sep_id = intern(&sep, &mut sym2id, &mut id2sym);

        // Build trie over u16 labels
        let mut build_nodes: Vec<BuildNode> = vec![BuildNode::default()]; // root = 0
        for s in &words {
            let tokens = tokenizer.segment(s);
            let n = tokens.len();
            for split in 0..=n {
                let mut seq: Vec<u16> = Vec::with_capacity(n + 1);
                for tk in tokens[..split].iter().rev() { seq.push(*sym2id.get(tk).expect("interned")); }
                seq.push(sep_id);
                for tk in &tokens[split..] { seq.push(*sym2id.get(tk).expect("interned")); }
                let mut node = 0usize;
                for &lab in &seq {
                    let next = if let Some(&id) = build_nodes[node].edges.get(&lab) {
                        id
                    } else {
                        let id = build_nodes.len();
                        build_nodes.push(BuildNode::default());
                        build_nodes[node].edges.insert(lab, id);
                        id
                    };
                    node = next;
                }
                build_nodes[node].terminal = true;
            }
        }

        // Freeze packed arrays
        let mut g_nodes: Vec<PackedNode> = Vec::with_capacity(build_nodes.len());
        let mut g_arcs: Vec<PackedArc> = Vec::new();
        for bn in &build_nodes {
            let offset = g_arcs.len() as u32;
            let degree = bn.edges.len() as u16;
            for (&label, &target) in bn.edges.iter() {
                g_arcs.push(PackedArc { label, target: target as u32 });
            }
            g_nodes.push(PackedNode { offset, degree, flags: if bn.terminal { 1 } else { 0 } });
        }

        let forward_opts = DictionaryOptions { case_fold, min_len, max_len, norm, tokenizer: tokenizer.clone() };
        let forward = FstDictionary::from_words_opts(words.clone(), forward_opts);
        Self { forward, g_nodes, g_arcs, sym2id, id2sym, sep, sep_id, tokenizer }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
        opts: DictionaryOptions,
    ) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut words: Vec<String> = Vec::new();
        for line in reader.lines() {
            let raw = line?;
            let mut s = normalize_with_mode(raw.trim(), opts.norm);
            if opts.case_fold {
                s = s.to_lowercase();
            }
            if s.is_empty() {
                continue;
            }
            let len = opts.tokenizer.segment(&s).len();
            if let Some(min) = opts.min_len
                && len < min
            {
                continue;
            }
            if let Some(max) = opts.max_len
                && len > max
            {
                continue;
            }
            words.push(s);
        }
        Ok(Self::from_words_opts(words, opts))
    }

    /// Serialize the GADDAG automaton and normalized word list to a CBOR file.
    /// Note: The tokenizer used for move generation is not serialized. Provide it when loading.
    pub fn to_gaddag_file<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        use std::io::BufWriter;
        let norm_mode: u8 = match self.forward.norm { NormalizationMode::NFC => 0, NormalizationMode::NFKC => 1 };
        // Collect all words from FST using an empty prefix automaton.
        let mut words: Vec<String> = Vec::new();
        {
            use fst::{IntoStreamer, Streamer, automaton::Str};
            let aut = Str::new("").starts_with();
            let mut stream = self.forward.set.search(aut).into_stream();
            while let Some(bytes) = stream.next() {
                let s = std::str::from_utf8(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("invalid utf8 in FST: {}", e)))?
                    .to_string();
                words.push(s);
            }
        }
        let nodes: Vec<PackedNodeDisk> = self
            .g_nodes
            .iter()
            .map(|n| PackedNodeDisk { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let arcs: Vec<PackedArcDisk> = self
            .g_arcs
            .iter()
            .map(|a| PackedArcDisk { label: a.label, target: a.target })
            .collect();
        let image = GaddagDiskImage {
            sep: self.sep.clone(),
            sep_id: self.sep_id,
            id2sym: self.id2sym.clone(),
            nodes,
            arcs,
            words,
            case_fold: self.forward.case_fold,
            norm_mode,
        };
        let f = std::fs::File::create(path)?;
        let mut w = BufWriter::new(f);
        ciborium::ser::into_writer(&image, &mut w)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    /// Serialize to CBOR bytes for embedding (e.g., `include_bytes!` in WASM bundles).
    pub fn to_gaddag_bytes(&self) -> std::io::Result<Vec<u8>> {
        let norm_mode: u8 = match self.forward.norm { NormalizationMode::NFC => 0, NormalizationMode::NFKC => 1 };
        let mut words: Vec<String> = Vec::new();
        {
            use fst::{IntoStreamer, Streamer, automaton::Str};
            let aut = Str::new("").starts_with();
            let mut stream = self.forward.set.search(aut).into_stream();
            while let Some(bytes) = stream.next() {
                let s = std::str::from_utf8(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("invalid utf8 in FST: {}", e)))?
                    .to_string();
                words.push(s);
            }
        }
        let nodes: Vec<PackedNodeDisk> = self
            .g_nodes
            .iter()
            .map(|n| PackedNodeDisk { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let arcs: Vec<PackedArcDisk> = self
            .g_arcs
            .iter()
            .map(|a| PackedArcDisk { label: a.label, target: a.target })
            .collect();
        let image = GaddagDiskImage { sep: self.sep.clone(), sep_id: self.sep_id, id2sym: self.id2sym.clone(), nodes, arcs, words, case_fold: self.forward.case_fold, norm_mode };
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&image, &mut buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(buf)
    }

    /// Load a serialized GADDAG CBOR file. Caller must supply the tokenizer that matches
    /// how the automaton was built.
    pub fn from_gaddag_file<P: AsRef<std::path::Path>>(
        path: P,
        tokenizer: TokenizerRef,
    ) -> std::io::Result<Self> {
        use std::io::BufReader;
        let f = std::fs::File::open(path)?;
        let r = BufReader::new(f);
        let image: GaddagDiskImage =
            ciborium::de::from_reader(r).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let g_nodes: Vec<PackedNode> = image
            .nodes
            .iter()
            .map(|n| PackedNode { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let g_arcs: Vec<PackedArc> = image
            .arcs
            .iter()
            .map(|a| PackedArc { label: a.label, target: a.target })
            .collect();
        let mut sym2id: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
        for (i, s) in image.id2sym.iter().enumerate() {
            sym2id.insert(s.clone(), i as u16);
        }
        let norm = match image.norm_mode { 0 => NormalizationMode::NFC, 1 => NormalizationMode::NFKC, _ => NormalizationMode::NFC };
        let forward_opts = DictionaryOptions { case_fold: image.case_fold, min_len: None, max_len: None, norm, tokenizer: tokenizer.clone() };
        let forward = FstDictionary::from_words_opts(image.words, forward_opts.clone());
        Ok(Self { forward, g_nodes, g_arcs, sym2id, id2sym: image.id2sym, sep: image.sep, sep_id: image.sep_id, tokenizer })
    }

    /// Load from CBOR bytes (e.g., embedded via `include_bytes!`).
    pub fn from_gaddag_bytes<D: AsRef<[u8]>>(
        bytes: D,
        tokenizer: TokenizerRef,
    ) -> std::io::Result<Self> {
        let cursor = std::io::Cursor::new(bytes.as_ref());
        let image: GaddagDiskImage =
            ciborium::de::from_reader(cursor).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let g_nodes: Vec<PackedNode> = image
            .nodes
            .iter()
            .map(|n| PackedNode { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let g_arcs: Vec<PackedArc> = image
            .arcs
            .iter()
            .map(|a| PackedArc { label: a.label, target: a.target })
            .collect();
        let mut sym2id: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
        for (i, s) in image.id2sym.iter().enumerate() { sym2id.insert(s.clone(), i as u16); }
        let norm = match image.norm_mode { 0 => NormalizationMode::NFC, 1 => NormalizationMode::NFKC, _ => NormalizationMode::NFC };
        let forward_opts = DictionaryOptions { case_fold: image.case_fold, min_len: None, max_len: None, norm, tokenizer: tokenizer.clone() };
        let forward = FstDictionary::from_words_opts(image.words, forward_opts.clone());
        Ok(Self { forward, g_nodes, g_arcs, sym2id, id2sym: image.id2sym, sep: image.sep, sep_id: image.sep_id, tokenizer })
    }

    pub fn root(&self) -> usize {
        0
    }
    pub fn sep_token(&self) -> &str {
        &self.sep
    }
    pub fn step_token(&self, node: usize, token: &str) -> Option<usize> {
        let &id = self.sym2id.get(token)?;
        self.step_by_id(node, id)
    }
    #[inline]
    fn step_by_id(&self, node: usize, sym_id: u16) -> Option<usize> {
        let n = *self.g_nodes.get(node)?;
        let slice = &self.g_arcs[n.offset as usize .. n.offset as usize + n.degree as usize];
        let mut lo = 0usize;
        let mut hi = slice.len();
        while lo < hi {
            let mid = (lo + hi) >> 1;
            let m = &slice[mid];
            if m.label < sym_id { lo = mid + 1; } else { hi = mid; }
        }
        if lo < slice.len() && slice[lo].label == sym_id { Some(slice[lo].target as usize) } else { None }
    }

    pub fn is_terminal(&self, node: usize) -> bool {
        self.g_nodes.get(node).map(|n| n.terminal()).unwrap_or(false)
    }

    /// Enumerate simple rightward suffixes from an anchor with given left context using rack letters.
    /// left_context is the contiguous string immediately to the left of anchor (not reversed).
    /// Returns full words (left_context + suffix) that are in the dictionary.
    pub fn enumerate_suffixes_simple(
        &self,
        left_context: &str,
        rack: &mut std::collections::HashMap<String, usize>,
        max_len: usize,
    ) -> Vec<String> {
        // compute pre node from left_context quickly
        let mut node = self.root();
        for token in self.tokenizer.segment(left_context).into_iter().rev() {
            let &id = if let Some(id) = self.sym2id.get(&token) { id } else { return vec![]; };
            if let Some(n2) = self.step_by_id(node, id) {
                node = n2;
            } else { return vec![]; }
        }
        if let Some(n2) = self.step_by_id(node, self.sep_id) { node = n2; } else { return vec![]; }

        // DFS on packed arcs
        let mut out = Vec::new();
        fn dfs(
            g: &GaddagDictionary,
            node: usize,
            built: &mut Vec<String>,
            rack: &mut std::collections::HashMap<String, usize>,
            out: &mut Vec<String>,
            left_context: &str,
            max_len: usize,
        ) {
            if built.len() >= max_len { return; }
            if !built.is_empty() && g.is_terminal(node) {
                let suffix = built.join("");
                out.push(format!("{}{}", left_context, suffix));
            }
            let ninfo = &g.g_nodes[node];
            let arcs = &g.g_arcs[ninfo.offset as usize .. ninfo.offset as usize + ninfo.degree as usize];
            for arc in arcs {
                if arc.label == g.sep_id { continue; }
                let sym = &g.id2sym[arc.label as usize];
                if rack.get(sym).copied().unwrap_or(0) > 0 {
                    { let c = rack.get_mut(sym).unwrap(); *c -= 1; }
                    built.push(sym.clone());
                    dfs(g, arc.target as usize, built, rack, out, left_context, max_len);
                    built.pop();
                    { let c = rack.get_mut(sym).unwrap(); *c += 1; }
                }
            }
        }
        dfs(self, node, &mut Vec::new(), rack, &mut out, left_context, max_len);
        out
    }

    pub fn step_symbol(&self, node: usize, sym: &str) -> Option<usize> {
        self.step_token(node, sym)
    }

    pub fn tokenizer(&self) -> &TokenizerRef {
        &self.tokenizer
    }
    // Small helpers for id-based paths
    pub fn symbol_id(&self, sym: &str) -> Option<u16> { self.sym2id.get(sym).copied() }
    pub fn id_to_symbol(&self, id: u16) -> &str { &self.id2sym[id as usize] }
    pub fn alphabet_len(&self) -> usize { self.id2sym.len() }
}

pub struct GaddagCursor<'a> {
    dict: &'a GaddagDictionary,
    pre: usize,
}

pub struct GaddagRight<'a> {
    dict: &'a GaddagDictionary,
    node: usize,
}

impl<'a> GaddagCursor<'a> {
    pub fn new(dict: &'a GaddagDictionary, left_context: &str) -> Option<Self> {
        let mut node = dict.root();
        for token in dict.tokenizer.segment(left_context).into_iter().rev() {
            node = dict.step_token(node, &token)?;
        }
        Some(Self { dict, pre: node })
    }
    // Fast path without tokenization
    pub fn new_from_tokens(dict: &'a GaddagDictionary, left_tokens: &[u16]) -> Option<Self> {
        let mut node = dict.root();
        for &id in left_tokens.iter().rev() {
            node = dict.step_by_id(node, id)?;
        }
        Some(Self { dict, pre: node })
    }
    pub fn step_left(&self, sym: &str) -> Option<Self> {
        let n = self.dict.step_token(self.pre, sym)?;
        Some(Self {
            dict: self.dict,
            pre: n,
        })
    }
    pub fn branch_right(&self) -> Option<GaddagRight<'a>> {
        let n = self.dict.step_token(self.pre, self.dict.sep_token())?;
        Some(GaddagRight {
            dict: self.dict,
            node: n,
        })
    }
    pub fn pre_node(&self) -> usize {
        self.pre
    }
}

impl<'a> GaddagRight<'a> {
    pub fn step(&self, sym: &str) -> Option<Self> {
        let n = self.dict.step_token(self.node, sym)?;
        Some(Self {
            dict: self.dict,
            node: n,
        })
    }
    // Step by symbol id (fast path)
    pub fn step_id(&self, sym_id: u16) -> Option<Self> {
        let n = self.dict.step_by_id(self.node, sym_id)?;
        Some(Self { dict: self.dict, node: n })
    }
    pub fn is_terminal(&self) -> bool {
        self.dict.is_terminal(self.node)
    }
    pub fn node(&self) -> usize {
        self.node
    }
}

impl Dictionary for GaddagDictionary {
    fn contains(&self, word: &str) -> bool {
        self.forward.contains(word)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        self.forward.has_prefix(prefix)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn boxed_clone(&self) -> Box<dyn Dictionary + Send + Sync> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct DictionaryOptions { /* moved */ }
impl Default for DictionaryOptions { fn default() -> Self { unreachable!("moved") } }
*/

// -------- Tests --------

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_normalization::UnicodeNormalization;

    fn rect(w: u32, h: u32) -> RectGridGeometry { RectGridGeometry::new(w, h) }

    #[test]
    fn version_smoke() {
        assert_eq!(engine_version().major, 0);
    }

    #[test]
    fn geometry_neighbors_edges_and_corners() {
        let g = rect(3, 3);
        // center (1,1) should have 4 neighbors
        let center = g.to_cell_id(Coord2D { x: 1, y: 1 }).unwrap();
        let ns = g.neighbors(center);
        assert_eq!(ns.len(), 4);
        // corner (0,0) should have 2 neighbors
        let c00 = g.to_cell_id(Coord2D { x: 0, y: 0 }).unwrap();
        assert_eq!(g.neighbors(c00).len(), 2);
        // edge (0,1) should have 3 neighbors
        let e01 = g.to_cell_id(Coord2D { x: 0, y: 1 }).unwrap();
        assert_eq!(g.neighbors(e01).len(), 3);
    }

    #[test]
    fn index_roundtrip() {
        let g = rect(10, 7);
        for y in 0..7i32 {
            for x in 0..10i32 {
                let c = Coord2D { x, y };
                let id = g.to_cell_id(c).unwrap();
                let back = g.from_cell_id(id).unwrap();
                assert_eq!(back, c);
            }
        }
    }

    #[test]
    fn rack_add_remove_capacity() {
        let mut r = Rack::default();
        let rs = 2;
        r.add(
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
            rs,
        )
        .unwrap();
        r.add(
            Tile {
                kind_id: "B".into(),
                mark: None,
            },
            rs,
        )
        .unwrap();
        assert!(matches!(
            r.add(
                Tile {
                    kind_id: "C".into(),
                    mark: None
                },
                rs
            ),
            Err(EngineError::RackCapacity)
        ));
        assert_eq!(r.len(), 2);
        let t = r.remove_at(0).unwrap();
        assert_eq!(t.kind_id, "A");
        assert_eq!(r.len(), 1);
    }

    fn make_counts() -> HashMap<TileKind, u32> {
        let a = TileKind {
            id: "A".into(),
            symbol: nfc("A"),
            score: 1,
            is_blank: false,
            aliases: vec![],
        };
        let b = TileKind {
            id: "B".into(),
            symbol: nfc("B"),
            score: 3,
            is_blank: false,
            aliases: vec![],
        };
        HashMap::from([(a, 2u32), (b, 1u32)])
    }

    #[test]
    fn bag_draw_depletes_and_is_deterministic() {
        let seed = 42;
        let mut bag1 = Bag::with_counts(make_counts(), seed);
        let mut bag2 = Bag::with_counts(make_counts(), seed);
        assert_eq!(bag1.remaining(), 3);
        let d1 = bag1.draw(3);
        let d2 = bag2.draw(3);
        assert_eq!(d1, d2);
        assert_eq!(bag1.remaining(), 0);
        assert!(bag1.draw_one().is_none());
    }

    #[test]
    fn bonus_default_is_identity() {
        let b = Bonus::default();
        assert_eq!(b.letter_mul, 1);
        assert_eq!(b.word_mul, 1);
        assert!(b.tags.is_empty());
    }

    #[test]
    fn state_new_and_preview() {
        // Build config
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 2);
        counts.insert("B".to_string(), 1);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 3,
                height: 3,
            },
            ruleset_id: "crossword_classic".into(),
            dictionary_id: "en_test".into(),
            rng_seed: 1,
            tile_counts: counts,
        };
        let state = GameState::new(&cfg, 2).unwrap();
        assert_eq!(state.board.geom.width, 3);
        assert_eq!(state.players.len(), 2);
        let c = state.board.geom.to_cell_id(Coord2D { x: 1, y: 1 }).unwrap();
        let draft = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let nb = state.preview(&draft).unwrap();
        assert_eq!(nb.cells[c.0 as usize].stack.len(), 1);
    }

    #[test]
    fn first_move_must_cover_center_and_contiguous() {
        let tileset = Tileset {
            tile_kinds: vec![TileKind {
                id: "A".into(),
                symbol: "A".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            }],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "crossword_classic".into(),
            dictionary_id: "en".into(),
            rng_seed: 1,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let rules = CrosswordRules::default();
        // invalid: not covering center
        let off = st.board.geom.to_cell_id(Coord2D { x: 0, y: 0 }).unwrap();
        let mv = MoveDraft {
            placements: vec![(
                off,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        assert!(rules.validate(&st, &mv).is_err());
        // valid: cover center
        let cen = CrosswordRules::center_cell(&st.board.geom);
        let mv = MoveDraft {
            placements: vec![(
                cen,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &v);
        assert!(sc.total >= 1);
        rules.commit(&mut st, v, &sc).unwrap();
    }

    #[test]
    fn anchor_requirement_and_contiguity() {
        // Prepare board with center tile placed
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 7,
                height: 7,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 2,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let rules = CrosswordRules::default();
        let c = CrosswordRules::center_cell(&st.board.geom);
        let first = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = rules.validate(&st, &first).unwrap();
        let sc1 = rules.score(&st, &v1);
        rules.commit(&mut st, v1, &sc1).unwrap();
        // invalid: two tiles not contiguous (gap)
        let left = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: st.board.geom.from_cell_id(c).unwrap().x - 2,
                y: st.board.geom.from_cell_id(c).unwrap().y,
            })
            .unwrap();
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: st.board.geom.from_cell_id(c).unwrap().x + 2,
                y: st.board.geom.from_cell_id(c).unwrap().y,
            })
            .unwrap();
        let mv = MoveDraft {
            placements: vec![
                (
                    left,
                    Tile {
                        kind_id: "B".into(),
                        mark: None,
                    },
                ),
                (
                    right,
                    Tile {
                        kind_id: "B".into(),
                        mark: None,
                    },
                ),
            ],
        };
        assert!(rules.validate(&st, &mv).is_err());
        // invalid: not adjacent to existing
        let far = st.board.geom.to_cell_id(Coord2D { x: 0, y: 0 }).unwrap();
        let mv = MoveDraft {
            placements: vec![(
                far,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            )],
        };
        assert!(rules.validate(&st, &mv).is_err());
        // valid: adjacent to center
        let adj = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: st.board.geom.from_cell_id(c).unwrap().x + 1,
                y: st.board.geom.from_cell_id(c).unwrap().y,
            })
            .unwrap();
        let mv = MoveDraft {
            placements: vec![(
                adj,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            )],
        };
        assert!(rules.validate(&st, &mv).is_ok());
    }

    #[test]
    fn scoring_with_bonuses_and_cross() {
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 3,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let rules = CrosswordRules::default();
        let center = CrosswordRules::center_cell(&st.board.geom);
        // Set word multiplier on center and letter multiplier on one neighbor
        st.board.bonuses.insert(
            center,
            Bonus {
                letter_mul: 1,
                word_mul: 2,
                tags: BTreeSet::new(),
            },
        );
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: st.board.geom.from_cell_id(center).unwrap().x + 1,
                y: st.board.geom.from_cell_id(center).unwrap().y,
            })
            .unwrap();
        st.board.bonuses.insert(
            right,
            Bonus {
                letter_mul: 3,
                word_mul: 1,
                tags: BTreeSet::new(),
            },
        );
        // First move: place A at center
        let mv1 = MoveDraft {
            placements: vec![(
                center,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        // base A(1) * W2 = 2
        assert_eq!(sc1.total, 2);
        rules.commit(&mut st, v1, &sc1).unwrap();
        // Second move: place B to the right (uses L3 at right, no word multiplier there)
        let mv2 = MoveDraft {
            placements: vec![(
                right,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            )],
        };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        // main word AB: A(1 existing) + B(3*3=9) = 10
        assert_eq!(sc2.main_score, 10);
        assert!(sc2.total >= 10);
    }

    #[test]
    fn dictionary_normalization_and_casefold() {
        // composed vs decomposed
        let composed = "Café".to_string();
        let decomposed = "Cafe\u{301}".nfc().collect::<String>();
        let dict = SetDictionary::from_words(vec![decomposed.clone()], false);
        assert!(dict.contains(&composed));
        // case fold
        let dict_cf = SetDictionary::from_words(vec!["café".to_string()], true);
        assert!(dict_cf.contains("CAFÉ"));
        let dict_no = SetDictionary::from_words(vec!["café".to_string()], false);
        assert!(!dict_no.contains("CAFÉ"));
    }

    #[test]
    fn dictionary_loader_from_file() {
        // Create a temporary word list
        let dir = std::env::temp_dir();
        let path = dir.join("tiletangle_dict_test.txt");
        let content = "# sample\nHELLO\nworld\n \nCafé\n";
        std::fs::write(&path, content).unwrap();
        let dict = SetDictionary::from_file(
            &path,
            DictionaryOptions {
                case_fold: true,
                min_len: Some(2),
                max_len: None,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(dict.contains("hello"));
        assert!(dict.contains("WORLD"));
        assert!(dict.contains("cafe\u{301}")); // decomposed
        assert!(!dict.contains("x"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn fst_dictionary_contains_and_prefix() {
        let dict = FstDictionary::from_words(
            vec!["AB".to_string(), "ABC".to_string(), "BEE".to_string()],
            true,
        );
        assert!(dict.contains("ab"));
        assert!(dict.has_prefix("ab"));
        assert!(dict.contains("abc"));
        assert!(!dict.contains("abd"));
        assert!(!dict.has_prefix("zz"));
    }

    #[test]
    fn gaddag_dictionary_basic() {
        let dict =
            GaddagDictionary::from_words(vec!["CARE".to_string(), "CARES".to_string()], true);
        assert!(dict.contains("care"));
        assert!(dict.has_prefix("ca"));
    }

    #[test]
    fn gaddag_forms_for_cares() {
        // Validate GADDAG encoded forms exist in the internal trie
        let gd = GaddagDictionary::from_words(vec!["CARES".to_string()], true);
        // helper to check presence of a sequence in internal g_nodes
        fn has_seq(gd: &GaddagDictionary, s: &str) -> bool {
            let mut node = gd.root();
            for token in gd.tokenizer().segment(s) {
                if let Some(nxt) = gd.step_token(node, &token) { node = nxt; } else { return false; }
            }
            gd.is_terminal(node)
        }
        // Our builder includes split positions 0..=n, so it includes "+CARES"
        assert!(has_seq(&gd, "+cares"));
        assert!(has_seq(&gd, "c+ares"));
        assert!(has_seq(&gd, "ac+res"));
        assert!(has_seq(&gd, "rac+es"));
        assert!(has_seq(&gd, "erac+s"));
        assert!(has_seq(&gd, "serac+"));
    }

    #[test]
    fn gaddag_id_cursor_and_step_id() {
        let gd = GaddagDictionary::from_words(vec!["AB".to_string()], true);
        let a = gd.symbol_id("a").expect("a id");
        let b = gd.symbol_id("b").expect("b id");
        let cur = GaddagCursor::new_from_tokens(&gd, &[a]).expect("cursor");
        let right = cur.branch_right().expect("branch");
        let r2 = right.step_id(b).expect("step b");
        assert!(r2.is_terminal());
    }

    #[test]
    fn gaddag_enumerate_suffixes_simple_ab() {
        let gd = GaddagDictionary::from_words(vec!["ab".to_string()], true);
        let mut rack: std::collections::HashMap<String, usize> =
            std::collections::HashMap::from([("a".to_string(), 1usize), ("b".to_string(), 1usize)]);
        let words = gd.enumerate_suffixes_simple("", &mut rack, 8);
        assert!(words.contains(&"ab".to_string()));
    }

    #[test]
    fn gaddag_normalization_and_casefold() {
        // composed vs decomposed + casefold behavior
        let composed = "Café".to_string();
        let decomposed = "Cafe\u{301}".nfc().collect::<String>();
        let gd = GaddagDictionary::from_words(vec![decomposed.clone()], true);
        assert!(gd.contains(&composed));
        assert!(gd.has_prefix("caf"));
    }

    #[test]
    fn dawg_dictionary_contains_and_prefix() {
        let dict = DawgDictionary::from_words(
            vec!["AB".to_string(), "ABC".to_string(), "BEE".to_string()],
            true,
        );
        assert!(dict.contains("ab"));
        assert!(dict.has_prefix("ab"));
        assert!(dict.contains("abc"));
        assert!(!dict.contains("abd"));
        assert!(!dict.has_prefix("zz"));
    }

    #[test]
    fn dawg_normalization_and_casefold() {
        let composed = "Café".to_string();
        let decomposed = "Cafe\u{301}".nfc().collect::<String>();
        let dict_cf = DawgDictionary::from_words(vec![decomposed.clone()], true);
        assert!(dict_cf.contains(&composed));
        assert!(dict_cf.has_prefix("caf"));
        let dict_no = DawgDictionary::from_words(vec!["café".to_string()], false);
        assert!(!dict_no.contains("CAFÉ"));
    }

    #[test]
    fn nfkc_contains_ligature_variant() {
        // Build dict with NFC words; use NFKC mode so ligatures map to base letters
        let opts = DictionaryOptions {
            case_fold: false,
            min_len: None,
            max_len: None,
            norm: NormalizationMode::NFKC,
            tokenizer: TokenizerRef::default(),
        };
        let dict = FstDictionary::from_words_opts(vec!["coffee".to_string()], opts);
        let ligature = "coﬀee"; // contains U+FB00 LIGATURE FF
        assert!(dict.contains(ligature));
        // Default NFC dictionary should not match the ligature
        let dict_nfc = FstDictionary::from_words(vec!["coffee".to_string()], false);
        assert!(!dict_nfc.contains(ligature));
    }

    #[test]
    fn nfkc_move_accepts_halfwidth() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![TileKind {
                id: "HALFPA".into(),
                symbol: "ﾊﾟ".into(),
                score: 3,
                is_blank: false,
                aliases: vec![],
            }],
        };
        let mut counts = HashMap::new();
        counts.insert("HALFPA".to_string(), 5);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "jp".into(),
            rng_seed: 3,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let opts = DictionaryOptions {
            case_fold: false,
            min_len: None,
            max_len: None,
            norm: NormalizationMode::NFKC,
            tokenizer: TokenizerRef::default(),
        };
        st.dictionary = Some(Box::new(FstDictionary::from_words_opts(
            vec!["パ".to_string()],
            opts,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        let center = CrosswordRules::center_cell(&st.board.geom);
        let mv = MoveDraft {
            placements: vec![(
                center,
                Tile {
                    kind_id: "HALFPA".into(),
                    mark: None,
                },
            )],
        };
        let validated = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &validated);
        assert!(sc.main_score > 0);
        assert_eq!(sc.main_word, "ﾊﾟ");
    }

    #[test]
    fn custom_tokenizer_groups_qu() {
        use std::sync::Arc;
        struct QuTokenizer;
        impl Tokenizer for QuTokenizer {
            fn segment(&self, text: &str) -> Vec<String> {
                let mut out = Vec::new();
                let mut chars = text.chars().peekable();
                while let Some(ch) = chars.next() {
                    if ch == 'q' && chars.peek() == Some(&'u') {
                        chars.next();
                        out.push("qu".to_string());
                    } else {
                        out.push(ch.to_string());
                    }
                }
                out
            }
        }
        let tokenizer = TokenizerRef::new(Arc::new(QuTokenizer));
        let opts = DictionaryOptions {
            tokenizer: tokenizer.clone(),
            ..Default::default()
        };
        let dawgd = DawgDictionary::from_words_opts(vec!["squid".to_string()], opts.clone());
        assert!(dawgd.has_prefix("squ"));
        assert!(dawgd.contains("squid"));
        assert!(!dawgd.has_prefix("sqz"));
        let gd = GaddagDictionary::from_words_opts(vec!["qu".to_string()], opts);
        assert!(gd.step_symbol(gd.root(), "qu").is_some());
    }

    #[test]
    fn gaddag_serialize_roundtrip_default() {
        use std::path::PathBuf;
        let gd = GaddagDictionary::from_words(vec!["CARE".to_string(), "CARES".to_string()], true);
        let path = PathBuf::from(std::env::temp_dir()).join("gaddag_test_default.cbor");
        gd.to_gaddag_file(&path).unwrap();
        let gd2 = GaddagDictionary::from_gaddag_file(&path, TokenizerRef::default()).unwrap();
        // Case-folded lookup via forward dictionary
        assert!(gd2.contains("cares"));
        assert!(gd2.contains("CARE"));
        // Basic membership checks are sufficient for roundtrip integrity
    }

    #[test]
    fn gaddag_serialize_roundtrip_custom_tokenizer() {
        use std::sync::Arc;
        struct QuTokenizer;
        impl Tokenizer for QuTokenizer {
            fn segment(&self, text: &str) -> Vec<String> {
                let mut out = Vec::new();
                let mut chars = text.chars().peekable();
                while let Some(ch) = chars.next() {
                    if ch == 'q' && chars.peek() == Some(&'u') {
                        chars.next();
                        out.push("qu".to_string());
                    } else {
                        out.push(ch.to_string());
                    }
                }
                out
            }
        }
        let tokenizer = TokenizerRef::new(Arc::new(QuTokenizer));
        let opts = DictionaryOptions {
            tokenizer: tokenizer.clone(),
            ..Default::default()
        };
        let gd = GaddagDictionary::from_words_opts(vec!["qu".to_string()], opts);
        let p = std::env::temp_dir().join("gaddag_test_qu.cbor");
        gd.to_gaddag_file(&p).unwrap();
        let gd2 = GaddagDictionary::from_gaddag_file(&p, tokenizer).unwrap();
        assert!(gd2.step_symbol(gd2.root(), "qu").is_some());
    }

    #[test]
    fn gaddag_bytes_roundtrip() {
        let gd = GaddagDictionary::from_words(vec!["AB".to_string(), "ABC".to_string()], true);
        let bytes = gd.to_gaddag_bytes().unwrap();
        let gd2 = GaddagDictionary::from_gaddag_bytes(bytes, TokenizerRef::default()).unwrap();
        assert!(gd2.contains("ab"));
        assert!(gd2.contains("abc"));
        // Transition checks are validated in non-serialized tests
    }

    #[test]
    fn plugin_pipeline_composition_and_score_bonus() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 1,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec!["AB".to_string()],
            true,
        )));
        let base = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        let rules = PluginRules::new(
            base,
            vec![
                Box::new(BasicActionsPlugin),
                Box::new(ScoreBonusPlugin { bonus: 5 }),
            ],
        );
        // Actions equivalent to placing AB on center row
        let c = CrosswordRules::center_cell(&st.board.geom);
        let cc = st.board.geom.from_cell_id(c).unwrap();
        let mut mv = UserMove::default();
        mv.actions.push(Action::Place {
            x: cc.x,
            y: cc.y,
            kind_id: "A".into(),
            mark: None,
        });
        mv.actions.push(Action::Place {
            x: cc.x + 1,
            y: cc.y,
            kind_id: "B".into(),
            mark: None,
        });
        let v = rules.validate_user_move(&st, mv).unwrap();
        let sc = rules.score_user_move(&st, &v);
        // base score: AB => 1 + 3 = 4; plugin bonus adds +5
        assert_eq!(sc.total, 9);
        rules.commit_user_move(&mut st, v, &sc).unwrap();
    }

    #[test]
    fn plugin_rejects_unsupported_action() {
        let tileset = Tileset {
            tile_kinds: vec![TileKind {
                id: "A".into(),
                symbol: "A".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            }],
        };
        let mut counts = std::collections::HashMap::new();
        counts.insert("A".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 3,
                height: 3,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 1,
            tile_counts: counts,
        };
        let st = GameState::new(&cfg, 2).unwrap();
        let base = CrosswordRules::default();
        let rules = PluginRules::new(base, vec![Box::new(BasicActionsPlugin)]);
        let mut mv = UserMove::default();
        mv.actions.push(Action::SwapRack {
            give: vec!["A".into()],
        });
        let err = rules.validate_user_move(&st, mv).unwrap_err();
        match err {
            EngineError::Config(_) => {}
            _ => panic!("expected config error"),
        }
    }

    #[test]
    fn rules_with_fst_dictionary() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 5,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec!["AB".to_string(), "B".to_string()],
            true,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        // First move: single A invalid (not in dict)
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert!(sc1.main_score < 0);
        // Place AB horizontally: allowed
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: st.board.geom.from_cell_id(c).unwrap().x + 1,
                y: st.board.geom.from_cell_id(c).unwrap().y,
            })
            .unwrap();
        let mv2 = MoveDraft {
            placements: vec![
                (
                    c,
                    Tile {
                        kind_id: "A".into(),
                        mark: None,
                    },
                ),
                (
                    right,
                    Tile {
                        kind_id: "B".into(),
                        mark: None,
                    },
                ),
            ],
        };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.total >= 0);
    }

    #[test]
    fn emoji_grapheme_word_scoring() {
        use std::collections::HashMap;
        let emoji = "👩‍🚀"; // woman astronaut (ZWJ sequence)
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "EM".into(),
                    symbol: emoji.into(),
                    score: 5,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("EM".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 11,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        // Dictionary contains A + emoji
        let word = format!("A{}", emoji);
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec![word.clone()],
            false,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        let c = CrosswordRules::center_cell(&st.board.geom);
        let cc = st.board.geom.from_cell_id(c).unwrap();
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: cc.x + 1,
                y: cc.y,
            })
            .unwrap();
        let mv = MoveDraft {
            placements: vec![
                (
                    c,
                    Tile {
                        kind_id: "A".into(),
                        mark: None,
                    },
                ),
                (
                    right,
                    Tile {
                        kind_id: "EM".into(),
                        mark: None,
                    },
                ),
            ],
        };
        let v = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &v);
        assert_eq!(sc.main_word, word);
        assert_eq!(sc.total, 1 + 5);
    }

    #[test]
    fn emoji_skin_tone_grapheme_mixed_with_letter() {
        use std::collections::HashMap;
        let thumbs = "👍🏽"; // thumbs up with medium skin tone
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "EM2".into(),
                    symbol: thumbs.into(),
                    score: 4,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("EM2".to_string(), 10);
        counts.insert("A".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 13,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        // Dict contains emoji + A
        let word = format!("{}A", thumbs);
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec![word.clone()],
            false,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        let c = CrosswordRules::center_cell(&st.board.geom);
        let cc = st.board.geom.from_cell_id(c).unwrap();
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: cc.x + 1,
                y: cc.y,
            })
            .unwrap();
        let mv = MoveDraft {
            placements: vec![
                (
                    c,
                    Tile {
                        kind_id: "EM2".into(),
                        mark: None,
                    },
                ),
                (
                    right,
                    Tile {
                        kind_id: "A".into(),
                        mark: None,
                    },
                ),
            ],
        };
        let v = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &v);
        assert_eq!(sc.main_word, word);
        assert_eq!(sc.total, 4 + 1);
    }

    #[test]
    fn blank_mapping_persists_and_scoring_unaffected() {
        use std::collections::HashMap;
        // Tiles: BL(blank,0), B(3)
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "BL".into(),
                    symbol: "_".into(),
                    score: 0,
                    is_blank: true,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("BL".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 12,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        // Add a 3L letter bonus on center; blank should remain 0 even with 3L
        let c = CrosswordRules::center_cell(&st.board.geom);
        st.board.bonuses.insert(
            c,
            Bonus {
                letter_mul: 3,
                word_mul: 1,
                tags: BTreeSet::new(),
            },
        );
        // Place BL mapped to 'A' only
        let rules_free = CrosswordRules {
            free_word_mode: true,
            ..Default::default()
        };
        let mv1 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "BL".into(),
                    mark: Some("A".into()),
                },
            )],
        };
        let v1 = rules_free.validate(&st, &mv1).unwrap();
        let sc1 = rules_free.score(&st, &v1);
        assert_eq!(sc1.total, 0);
        rules_free.commit(&mut st, v1, &sc1).unwrap();
        // Persisted mapping
        let top = st.board.cells[c.0 as usize].stack.last().unwrap();
        assert_eq!(top.kind_id, "BL");
        assert_eq!(top.mark.as_deref(), Some("A"));
        // Next move: place B to the right to form "AB"; score should be 3
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec!["AB".to_string()],
            true,
        )));
        let cc = st.board.geom.from_cell_id(c).unwrap();
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: cc.x + 1,
                y: cc.y,
            })
            .unwrap();
        let rules = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        let mv2 = MoveDraft {
            placements: vec![(
                right,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            )],
        };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert_eq!(sc2.main_word, "AB");
        assert_eq!(sc2.total, 3);
    }

    #[test]
    fn stacking_allows_overlay_and_forbids_same_symbol() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 1,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let rules = CrosswordRules {
            free_word_mode: true,
            stacking_enabled: true,
            stacking_max_height: 2,
            ..Default::default()
        };
        // Place A at center
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        rules.commit(&mut st, v1, &sc1).unwrap();
        // Overlay B on A: allowed
        let mv2 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            )],
        };
        let _ = rules.validate(&st, &mv2).unwrap();
        // Overlay A on A: forbidden when forbid_same_symbol_overlay=true
        let mv3 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        assert!(rules.validate(&st, &mv3).is_err());
    }

    #[test]
    fn stacking_sum_stack_scoring() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 2,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let rules = CrosswordRules {
            free_word_mode: true,
            stacking_enabled: true,
            stacking_max_height: 3,
            stacking_scoring: StackScoring::SumStack,
            ..Default::default()
        };
        // Place A at center
        let c = CrosswordRules::center_cell(&st.board.geom);
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: st.board.geom.from_cell_id(c).unwrap().x + 1,
                y: st.board.geom.from_cell_id(c).unwrap().y,
            })
            .unwrap();
        let mv1 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        rules.commit(&mut st, v1, &sc1).unwrap();
        // Overlay B on A at center and place A at right to form BA (visible BA)
        // With SumStack, the score for the stacked cell should be A(1) + B(3) = 4 (no bonuses)
        let mv2 = MoveDraft {
            placements: vec![
                (
                    c,
                    Tile {
                        kind_id: "B".into(),
                        mark: None,
                    },
                ),
                (
                    right,
                    Tile {
                        kind_id: "A".into(),
                        mark: None,
                    },
                ),
            ],
        };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.total >= 5); // at least 4 (stack) + 1 (A) = 5
    }

    #[test]
    fn rtl_horizontal_word_dictionary_check() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "xx".into(),
            rng_seed: 1,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        // Dictionary allows BA (right-to-left word)
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec!["BA".to_string()],
            true,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            reading_dir: ReadingDirection::RTL,
            ..Default::default()
        };
        // First move: place A at center using free mode commit
        let free_rules = CrosswordRules {
            free_word_mode: true,
            ..Default::default()
        };
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = free_rules.validate(&st, &mv1).unwrap();
        let sc1 = free_rules.score(&st, &v1);
        free_rules.commit(&mut st, v1, &sc1).unwrap();
        // Second move: place B to the right; physical "AB" but RTL read as "BA"
        let c_coord = st.board.geom.from_cell_id(c).unwrap();
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: c_coord.x + 1,
                y: c_coord.y,
            })
            .unwrap();
        let mv2 = MoveDraft {
            placements: vec![(
                right,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            )],
        };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.total >= 0);
        assert_eq!(sc2.main_word, "BA");
    }

    #[test]
    fn cross_word_validation_with_dict() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 7,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        // dict only allows AB
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec!["AB".to_string()],
            true,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        // First move: place A at center
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert!(sc1.main_score < 0); // A alone not in dict
        // Still commit A to set up cross check
        let free_rules = CrosswordRules {
            free_word_mode: true,
            ..Default::default()
        };
        let v1b = free_rules.validate(&st, &mv1).unwrap();
        let sc1b = free_rules.score(&st, &v1b);
        free_rules.commit(&mut st, v1b, &sc1b).unwrap();
        // Second move: place B to the right to form main word "AB"
        let c_coord = st.board.geom.from_cell_id(c).unwrap();
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: c_coord.x + 1,
                y: c_coord.y,
            })
            .unwrap();
        let mv2 = MoveDraft {
            placements: vec![(
                right,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            )],
        };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.total >= 0); // cross word AB is valid
    }

    #[test]
    fn dictionary_integration_in_rules() {
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "A".into(),
                    symbol: "A".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "B".into(),
                    symbol: "B".into(),
                    score: 3,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 5,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        // Dictionary only allows "AB"
        st.dictionary = Some(Box::new(SetDictionary::from_words(
            vec!["AB".to_string()],
            true,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            ..Default::default()
        };
        // First move: A at center not in dict alone → scoring returns sentinel (-1)
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft {
            placements: vec![(
                c,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
        };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert_eq!(sc1.main_score, -1);
        // Place AB horizontally: should be allowed
        let right = st
            .board
            .geom
            .to_cell_id(Coord2D {
                x: st.board.geom.from_cell_id(c).unwrap().x + 1,
                y: st.board.geom.from_cell_id(c).unwrap().y,
            })
            .unwrap();
        let mv2 = MoveDraft {
            placements: vec![
                (
                    c,
                    Tile {
                        kind_id: "A".into(),
                        mark: None,
                    },
                ),
                (
                    right,
                    Tile {
                        kind_id: "B".into(),
                        mark: None,
                    },
                ),
            ],
        };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.total > 0);
    }

    #[test]
    fn rtl_hebrew_word_validates() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "SHIN".into(),
                    symbol: "ש".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "LAMED".into(),
                    symbol: "ל".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "VAV".into(),
                    symbol: "ו".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "MEMF".into(),
                    symbol: "ם".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("SHIN".to_string(), 4);
        counts.insert("LAMED".to_string(), 4);
        counts.insert("VAV".to_string(), 4);
        counts.insert("MEMF".to_string(), 4);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 7,
                height: 7,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "he".into(),
            rng_seed: 7,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec!["שלום".to_string()],
            false,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            reading_dir: ReadingDirection::RTL,
            ..Default::default()
        };
        let center = CrosswordRules::center_cell(&st.board.geom);
        let base = st.board.geom.from_cell_id(center).unwrap();
        let placements = vec![
            (
                st.board
                    .geom
                    .to_cell_id(Coord2D {
                        x: base.x - 2,
                        y: base.y,
                    })
                    .unwrap(),
                Tile {
                    kind_id: "MEMF".into(),
                    mark: None,
                },
            ),
            (
                st.board
                    .geom
                    .to_cell_id(Coord2D {
                        x: base.x - 1,
                        y: base.y,
                    })
                    .unwrap(),
                Tile {
                    kind_id: "VAV".into(),
                    mark: None,
                },
            ),
            (
                center,
                Tile {
                    kind_id: "LAMED".into(),
                    mark: None,
                },
            ),
            (
                st.board
                    .geom
                    .to_cell_id(Coord2D {
                        x: base.x + 1,
                        y: base.y,
                    })
                    .unwrap(),
                Tile {
                    kind_id: "SHIN".into(),
                    mark: None,
                },
            ),
        ];
        let mv = MoveDraft { placements };
        let validated = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &validated);
        assert_eq!(sc.main_word, "שלום");
        assert!(sc.main_score > 0);
    }

    #[test]
    fn rtl_arabic_word_validates() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind {
                    id: "SEEN".into(),
                    symbol: "س".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "LAM".into(),
                    symbol: "ل".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "ALEF".into(),
                    symbol: "ا".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "MEEM".into(),
                    symbol: "م".into(),
                    score: 1,
                    is_blank: false,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("SEEN".to_string(), 4);
        counts.insert("LAM".to_string(), 4);
        counts.insert("ALEF".to_string(), 4);
        counts.insert("MEEM".to_string(), 4);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 7,
                height: 7,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "ar".into(),
            rng_seed: 11,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(
            vec!["سلام".to_string()],
            false,
        )));
        let rules = CrosswordRules {
            free_word_mode: false,
            reading_dir: ReadingDirection::RTL,
            ..Default::default()
        };
        let center = CrosswordRules::center_cell(&st.board.geom);
        let base = st.board.geom.from_cell_id(center).unwrap();
        let placements = vec![
            (
                st.board
                    .geom
                    .to_cell_id(Coord2D {
                        x: base.x - 2,
                        y: base.y,
                    })
                    .unwrap(),
                Tile {
                    kind_id: "MEEM".into(),
                    mark: None,
                },
            ),
            (
                st.board
                    .geom
                    .to_cell_id(Coord2D {
                        x: base.x - 1,
                        y: base.y,
                    })
                    .unwrap(),
                Tile {
                    kind_id: "ALEF".into(),
                    mark: None,
                },
            ),
            (
                center,
                Tile {
                    kind_id: "LAM".into(),
                    mark: None,
                },
            ),
            (
                st.board
                    .geom
                    .to_cell_id(Coord2D {
                        x: base.x + 1,
                        y: base.y,
                    })
                    .unwrap(),
                Tile {
                    kind_id: "SEEN".into(),
                    mark: None,
                },
            ),
        ];
        let mv = MoveDraft { placements };
        let validated = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &validated);
        assert_eq!(sc.main_word, "سلام");
        assert!(sc.main_score > 0);
    }
}
