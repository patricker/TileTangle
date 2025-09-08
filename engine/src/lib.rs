//! TileTangle Engine — Core Model (Phase 1)

use rand::{Rng, SeedableRng, rngs::StdRng};
use fst::{Automaton, Streamer};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

// -------- Errors --------

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid coordinates")]
    InvalidCoordinates,
    #[error("invalid cell id")]
    InvalidCell,
    #[error("collision at cell {0:?}")]
    Collision(CellId),
    #[error("rack capacity exceeded")]
    RackCapacity,
    #[error("bag is empty")]
    BagEmpty,
    #[error("config error: {0}")]
    Config(&'static str),
}

// -------- Version --------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub fn engine_version() -> EngineVersion {
    EngineVersion {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

// -------- Symbols & Tiles --------

pub type Symbol = String; // NFC-normalized grapheme string

pub fn nfc<S: AsRef<str>>(s: S) -> Symbol {
    s.as_ref().nfc().collect()
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub kind_id: String,
    pub mark: Option<String>,
}

// -------- Board Geometry --------

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CellId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Coord2D {
    pub x: i32,
    pub y: i32,
}

pub trait BoardGeometry {
    fn neighbors(&self, id: CellId) -> SmallVec<[CellId; 4]>;
    fn to_cell_id(&self, c: Coord2D) -> Option<CellId>;
    #[allow(clippy::wrong_self_convention)]
    fn from_cell_id(&self, id: CellId) -> Option<Coord2D>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone)]
pub struct RectGridGeometry {
    pub width: u32,
    pub height: u32,
}

impl RectGridGeometry {
    fn index(&self, c: Coord2D) -> Option<u32> {
        if c.x < 0 || c.y < 0 {
            return None;
        }
        let (x, y) = (c.x as u32, c.y as u32);
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }
}

impl BoardGeometry for RectGridGeometry {
    fn neighbors(&self, id: CellId) -> SmallVec<[CellId; 4]> {
        let mut out: SmallVec<[CellId; 4]> = SmallVec::new();
        if let Some(c) = self.from_cell_id(id) {
            let dirs = [
                Coord2D { x: c.x - 1, y: c.y },
                Coord2D { x: c.x + 1, y: c.y },
                Coord2D { x: c.x, y: c.y - 1 },
                Coord2D { x: c.x, y: c.y + 1 },
            ];
            for d in dirs {
                if let Some(n) = self.to_cell_id(d) {
                    out.push(n);
                }
            }
        }
        out
    }

    fn to_cell_id(&self, c: Coord2D) -> Option<CellId> {
        self.index(c).map(CellId)
    }

    fn from_cell_id(&self, id: CellId) -> Option<Coord2D> {
        let i = id.0;
        if i >= self.width * self.height {
            return None;
        }
        let y = i / self.width;
        let x = i % self.width;
        Some(Coord2D {
            x: x as i32,
            y: y as i32,
        })
    }

    fn len(&self) -> usize {
        (self.width * self.height) as usize
    }
}

// -------- Board & Bonuses --------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bonus {
    pub letter_mul: i8, // default 1
    pub word_mul: i8,   // default 1
    pub tags: BTreeSet<String>,
}

impl Default for Bonus {
    fn default() -> Self {
        Self {
            letter_mul: 1,
            word_mul: 1,
            tags: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cell {
    pub stack: Vec<Tile>,
}

#[derive(Debug, Clone)]
pub struct Board<G: BoardGeometry> {
    pub geom: G,
    pub cells: Vec<Cell>,
    pub bonuses: HashMap<CellId, Bonus>,
}

impl<G: BoardGeometry> Board<G> {
    pub fn new(geom: G) -> Self {
        let len = geom.len();
        Self {
            geom,
            cells: vec![Cell::default(); len],
            bonuses: HashMap::new(),
        }
    }
}

// -------- Rack & Bag --------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rack {
    pub tiles: Vec<Tile>,
}

impl Rack {
    pub fn add(&mut self, t: Tile, rack_size: usize) -> Result<(), EngineError> {
        if self.tiles.len() >= rack_size {
            return Err(EngineError::RackCapacity);
        }
        self.tiles.push(t);
        Ok(())
    }
    pub fn remove_at(&mut self, i: usize) -> Option<Tile> {
        if i < self.tiles.len() {
            Some(self.tiles.remove(i))
        } else {
            None
        }
    }
    pub fn len(&self) -> usize {
        self.tiles.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Tileset {
    pub tile_kinds: Vec<TileKind>,
}

#[derive(Debug, Clone)]
pub struct Bag {
    pub counts: HashMap<TileKind, u32>,
    rng: StdRng,
}

impl Bag {
    pub fn from_tileset(ts: &Tileset, seed: u64) -> Self {
        let mut counts = HashMap::new();
        for tk in &ts.tile_kinds {
            // Use aliases field as count holder if provided via serde? Instead rely on an implicit count=1 if absent.
            // For Phase 1 tests, we will define counts explicitly when creating the bag.
            counts.insert(tk.clone(), 0);
        }
        Self {
            counts,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn with_counts(counts: HashMap<TileKind, u32>, seed: u64) -> Self {
        Self {
            counts,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn remaining(&self) -> u32 {
        self.counts.values().copied().sum()
    }

    pub fn draw(&mut self, n: usize) -> Vec<Tile> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(t) = self.draw_one() {
                out.push(t);
            }
        }
        out
    }

    pub fn draw_one(&mut self) -> Option<Tile> {
        let total = self.remaining();
        if total == 0 {
            return None;
        }
        let choice = self.rng.gen_range(0..total);
        // Iterate in deterministic order by kind id
        let mut items: Vec<(&TileKind, &u32)> =
            self.counts.iter().filter(|(_, c)| **c > 0).collect();
        items.sort_by(|(k1, _), (k2, _)| k1.id.cmp(&k2.id));
        let mut acc = 0u32;
        let mut selected_key: Option<TileKind> = None;
        for (k, c) in items {
            acc += *c;
            if choice < acc {
                selected_key = Some(k.clone());
                break;
            }
        }
        if let Some(key) = selected_key {
            let cnt = self.counts.get_mut(&key).unwrap();
            *cnt -= 1;
            Some(Tile {
                kind_id: key.id.clone(),
                mark: None,
            })
        } else {
            None
        }
    }
}

// -------- Players & Game State --------

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PlayerId(pub usize);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Player {
    pub rack: Rack,
    pub score: i32,
}

#[derive(Debug, Clone)]
pub struct RectBoardLayout {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub tileset: Tileset,
    pub rack_size: usize,
    pub board_layout: RectBoardLayout,
    pub ruleset_id: String,
    pub dictionary_id: String,
    pub rng_seed: u64,
    /// (optional) counts per tile kind id
    pub tile_counts: HashMap<String, u32>,
}

#[derive()]
pub struct GameState {
    pub board: Board<RectGridGeometry>,
    pub players: Vec<Player>,
    pub to_move: PlayerId,
    pub bag: Bag,
    pub turn_num: u32,
    pub tileset: Tileset,
    pub dictionary: Option<Box<dyn Dictionary + Send + Sync>>, 
}

impl GameState {
    pub fn new(config: &GameConfig, players: usize) -> Result<Self, EngineError> {
        if players == 0 {
            return Err(EngineError::Config("at least 1 player"));
        }
        let geom = RectGridGeometry {
            width: config.board_layout.width,
            height: config.board_layout.height,
        };
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

        let players_vec = (0..players).map(|_| Player::default()).collect();
        Ok(Self {
            board,
            players: players_vec,
            to_move: PlayerId(0),
            bag,
            turn_num: 0,
            tileset,
            dictionary: None,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveDraft {
    pub placements: Vec<(CellId, Tile)>,
}

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

#[derive(Debug, Clone)]
pub struct CrosswordRules {
    pub free_word_mode: bool,
    pub bingo_bonus: i32,
    pub require_center_first_move: bool,
}

impl Default for CrosswordRules {
    fn default() -> Self {
        Self {
            free_word_mode: true,
            bingo_bonus: 50,
            require_center_first_move: true,
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
            for n in board.geom.neighbors(*id) {
                if Self::cell_has_tile(board, n) {
                    return true;
                }
            }
        }
        false
    }

    fn tileset_lookup_score_symbol<'a>(tileset: &'a Tileset, kind_id: &str) -> (i16, &'a str) {
        for tk in &tileset.tile_kinds {
            if tk.id == kind_id {
                return (tk.score, tk.symbol.as_str());
            }
        }
        (0, "?")
    }

    fn form_word(
        board: &Board<RectGridGeometry>,
        tileset: &Tileset,
        start: Coord2D,
        dir: (i32, i32),
        placed: &std::collections::HashSet<CellId>,
    ) -> (String, i32) {
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
        // build string and score
        let mut word = String::new();
        let mut score: i32 = 0;
        let mut word_mul: i32 = 1;
        loop {
            if let Some(id) = board.geom.to_cell_id(c)
                && Self::cell_has_tile(board, id)
            {
                let cell = &board.cells[id.0 as usize];
                let tile = cell.stack.last().unwrap();
                let (ls, sym) = Self::tileset_lookup_score_symbol(tileset, &tile.kind_id);
                word.push_str(sym);
                let mut add = ls as i32;
                if placed.contains(&id)
                    && let Some(b) = board.bonuses.get(&id)
                {
                    add *= b.letter_mul as i32;
                    word_mul *= b.word_mul as i32;
                }
                score += add;
                c = Coord2D {
                    x: c.x + dir.0,
                    y: c.y + dir.1,
                };
                continue;
            }
            break;
        }
        (word, score * word_mul)
    }
}

impl Rules for CrosswordRules {
    fn validate(&self, state: &GameState, draft: &MoveDraft) -> Result<ValidatedMove, EngineError> {
        if draft.placements.is_empty() {
            return Err(EngineError::Config("no tiles placed"));
        }
        // no collisions: cannot place on occupied cells
        for (cid, _) in &draft.placements {
            if CrosswordRules::cell_has_tile(&state.board, *cid) {
                return Err(EngineError::Collision(*cid));
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
        let (main_word, main_score) =
            Self::form_word(&temp_board, &state.tileset, start_coord, dir, &placed_ids);
        // cross words
        let pdir = if mv.line_is_row { (0, 1) } else { (1, 0) };
        let mut cross_words = Vec::new();
        for (cid, _) in &mv.placements {
            let c = temp_board.geom.from_cell_id(*cid).unwrap();
            // Only if neighbors in perpendicular direction form a word length > 1
            // Build word centered at c in pdir
            let (w, s) = Self::form_word(&temp_board, &state.tileset, c, pdir, &placed_ids);
            if w.len() > 1 {
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
        let drawn = state.bag.draw(draw_n);
        state.players[pid].rack.tiles.extend(drawn);
        // Advance turn
        state.turn_num += 1;
        state.to_move = PlayerId((pid + 1) % state.players.len());
        Ok(())
    }
}

// -------- Move Generation (Phase 12 start) --------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateMove {
    pub placements: Vec<(CellId, Tile)>,
    pub word: String,
    pub score: i32,
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
            if !cell.stack.is_empty() { continue; }
            let id = CellId(idx as u32);
            if CrosswordRules::adjacent_to_existing(&state.board, &[id]) {
                anchors.push(id);
            }
        }
    }

    fn get_symbol<'a>(tileset: &'a Tileset, kind_id: &str) -> Option<(&'a TileKind, &'a str)> {
        for tk in &tileset.tile_kinds { if tk.id == kind_id { return Some((tk, tk.symbol.as_str())); } }
        None
    }

    // build helper to read cell including overlay
    #[derive(Default, Clone)]
    struct Overlay(std::collections::HashMap<CellId, Tile>);
    impl Overlay {
        fn get<'a>(&'a self, id: CellId, state: &'a GameState) -> Option<&'a Tile> {
            if let Some(t) = self.0.get(&id) { return Some(t); }
            let cell = &state.board.cells[id.0 as usize];
            cell.stack.last()
        }
    }

    fn vertical_word(state: &GameState, ov: &Overlay, at: CellId) -> String {
        let c0 = state.board.geom.from_cell_id(at).unwrap();
        // move up
        let mut c = c0;
        loop {
            let prev = Coord2D { x: c.x, y: c.y - 1 };
            if let Some(id) = state.board.geom.to_cell_id(prev) {
                if ov.get(id, state).is_some() { c = prev; continue; }
            }
            break;
        }
        let mut s = String::new();
        loop {
            if let Some(id) = state.board.geom.to_cell_id(c) {
                if let Some(tile) = ov.get(id, state) {
                    let (_, sym) = CrosswordRules::tileset_lookup_score_symbol(&state.tileset, &tile.kind_id);
                    s.push_str(sym);
                    c = Coord2D { x: c.x, y: c.y + 1 };
                    continue;
                }
            }
            break;
        }
        s
    }

    // DFS to the right from anchor only (simplified); ensure anchor included
    fn dfs_from(
        state: &GameState,
        rules: &impl Rules,
        anchor: CellId,
        rack: &mut std::collections::HashMap<String, usize>,
        built: String,
        pos: Coord2D,
        used: &mut Vec<(CellId, Tile)>,
        out: &mut Vec<CandidateMove>,
        max_len: usize,
    ) {
        if built.chars().count() >= max_len { return; }
        // if cell has fixed tile, append and continue
        if let Some(id) = state.board.geom.to_cell_id(pos) {
            let cell = &state.board.cells[id.0 as usize];
            if let Some(t) = cell.stack.last() {
                let mut nb = built.clone();
                let (_, sym) = CrosswordRules::tileset_lookup_score_symbol(&state.tileset, &t.kind_id);
                nb.push_str(sym);
                let next = Coord2D { x: pos.x + 1, y: pos.y };
                dfs_from(state, rules, anchor, rack, nb, next, used, out, max_len);
                return;
            }
        } else { return; }

        // Try placing from rack
        for (kind_id, cnt) in rack.clone() { // iterate snapshot
            if cnt == 0 { continue; }
            // place here
            let id = state.board.geom.to_cell_id(pos).unwrap();
            // Only place if empty
            if !state.board.cells[id.0 as usize].stack.is_empty() { continue; }
            // Resolve symbol
            let Some((tk, sym)) = get_symbol(&state.tileset, &kind_id) else { continue; };
            // Cross-check vertical
            let mut ov = Overlay::default();
            for (cid, tile) in used.iter() { ov.0.insert(*cid, tile.clone()); }
            ov.0.insert(id, Tile { kind_id: kind_id.clone(), mark: None });
            let vword = vertical_word(state, &ov, id);
            if vword.chars().count() > 1 {
                if let Some(dict) = &state.dictionary {
                    if !dict.contains(&vword) { continue; }
                }
            }
            // Append and recurse / also consider committing as a move end
            let mut nb = built.clone(); nb.push_str(sym);
            // Prepare used/rack
            *rack.get_mut(&kind_id).unwrap() -= 1;
            used.push((id, Tile { kind_id: kind_id.clone(), mark: None }));

            // Attempt to finalize this sequence as a play covering anchor
            // Build draft and validate+score
            if used.iter().any(|(cid, _)| *cid == anchor) || state.board.cells[anchor.0 as usize].stack.last().is_some() {
                let draft = MoveDraft { placements: used.clone() };
                if let Ok(v) = rules.validate(state, &draft) {
                    let sc = rules.score(state, &v);
                    if sc.total >= 0 { // valid dict words
                        out.push(CandidateMove { placements: used.clone(), word: sc.main_word.clone(), score: sc.total });
                    }
                }
            }

            // Recurse to the right
            let next = Coord2D { x: pos.x + 1, y: pos.y };
            dfs_from(state, rules, anchor, rack, nb, next, used, out, max_len);

            // backtrack
            used.pop();
            *rack.get_mut(&kind_id).unwrap() += 1;
        }
    }

    let mut out: Vec<CandidateMove> = Vec::new();
    for a in anchors {
        let start = state.board.geom.from_cell_id(a).unwrap();
        let mut rack_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for k in rack { *rack_counts.entry(k.clone()).or_default() += 1; }
        dfs_from(state, rules, a, &mut rack_counts, String::new(), start, &mut Vec::new(), &mut out, max_len);
    }
    // de-duplicate identical placement sets (simple)
    out.sort_by_key(|cm| (cm.score, cm.word.clone(), cm.placements.len()));
    out
}

impl Player {
    fn rack_size(&self) -> Option<usize> {
        Some(7)
    }
}

// -------- Dictionary Engine (Phase 3) --------

pub trait Dictionary {
    fn contains(&self, word: &str) -> bool;
    fn has_prefix(&self, _prefix: &str) -> bool {
        false
    }
}

#[derive(Debug, Clone, Default)]
pub struct SetDictionary {
    words: std::collections::HashSet<String>,
    case_fold: bool,
}

impl SetDictionary {
    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
        opts: DictionaryOptions,
    ) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let mut set = std::collections::HashSet::new();
        let reader = BufReader::new(f);
        for line in reader.lines() {
            let s = line?;
            let s = s.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let mut w = nfc(s);
            if opts.case_fold {
                w = w.to_lowercase();
            }
            let len = w.chars().count();
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
            set.insert(w);
        }
        Ok(Self {
            words: set,
            case_fold: opts.case_fold,
        })
    }
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut set = std::collections::HashSet::new();
        for w in iter {
            let mut s = nfc(w.into());
            if case_fold {
                s = s.to_lowercase();
            }
            set.insert(s);
        }
        Self {
            words: set,
            case_fold,
        }
    }
}

impl Dictionary for SetDictionary {
    fn contains(&self, word: &str) -> bool {
        let mut s = nfc(word);
        if self.case_fold {
            s = s.to_lowercase();
        }
        self.words.contains(&s)
    }
}

#[derive(Debug, Clone)]
pub struct FstDictionary {
    set: fst::Set<Vec<u8>>,
    case_fold: bool,
}

impl FstDictionary {
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
        Self { set, case_fold }
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
            let mut w = nfc(s);
            if opts.case_fold {
                w = w.to_lowercase();
            }
            let len = w.chars().count();
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
        Ok(Self { set, case_fold: opts.case_fold })
    }
}

impl Dictionary for FstDictionary {
    fn contains(&self, word: &str) -> bool {
        let mut s = nfc(word);
        if self.case_fold {
            s = s.to_lowercase();
        }
        self.set.contains(&s)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        use fst::{automaton::Str, IntoStreamer};
        let mut p = nfc(prefix);
        if self.case_fold {
            p = p.to_lowercase();
        }
        let aut = Str::new(&p).starts_with();
        let mut stream = self.set.search(aut).into_stream();
        stream.next().is_some()
    }
}

#[derive(Debug, Clone)]
pub struct GaddagDictionary {
    // Forward lexicon for contains/has_prefix
    forward: FstDictionary,
    // GADDAG trie for move generation (internal, not yet used by Rules)
    g_nodes: Vec<GNode>,
    sep: char,
}

#[derive(Debug, Clone, Default)]
struct GNode {
    // transitions on chars (letters or separator)
    edges: std::collections::HashMap<char, usize>,
    terminal: bool,
}

impl GaddagDictionary {
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let sep = '+';
        let mut g_nodes = vec![GNode::default()]; // root at 0
        let mut words: Vec<String> = Vec::new();
        for w in iter.into_iter() {
            let mut s = nfc(w.into());
            if case_fold {
                s = s.to_lowercase();
            }
            if s.is_empty() {
                continue;
            }
            words.push(s.clone());
            let chars: Vec<char> = s.chars().collect();
            let n = chars.len();
            for split in 0..=n {
                let prefix = &chars[..split];
                let suffix = &chars[split..];
                // Build REV(prefix) + sep + suffix
                let mut seq: Vec<char> = prefix.iter().rev().copied().collect();
                seq.push(sep);
                seq.extend_from_slice(suffix);
                // insert into trie
                let mut node = 0usize;
                for ch in seq {
                    let next = if let Some(&id) = g_nodes[node].edges.get(&ch) {
                        id
                    } else {
                        let id = g_nodes.len();
                        g_nodes.push(GNode::default());
                        g_nodes[node].edges.insert(ch, id);
                        id
                    };
                    node = next;
                }
                g_nodes[node].terminal = true;
            }
        }
        let forward = FstDictionary::from_words(words, case_fold);
        Self { forward, g_nodes, sep }
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
            let mut s = nfc(line?.trim().to_string());
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if opts.case_fold {
                s = s.to_lowercase();
            }
            let len = s.chars().count();
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
        Ok(Self::from_words(words, opts.case_fold))
    }
}

impl Dictionary for GaddagDictionary {
    fn contains(&self, word: &str) -> bool {
        self.forward.contains(word)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        self.forward.has_prefix(prefix)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DictionaryOptions {
    pub case_fold: bool,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
}

// -------- Tests --------

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_normalization::UnicodeNormalization;

    fn rect(w: u32, h: u32) -> RectGridGeometry {
        RectGridGeometry {
            width: w,
            height: h,
        }
    }

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
        let dict = FstDictionary::from_words(vec!["AB".to_string(), "ABC".to_string(), "BEE".to_string()], true);
        assert!(dict.contains("ab"));
        assert!(dict.has_prefix("ab"));
        assert!(dict.contains("abc"));
        assert!(!dict.contains("abd"));
        assert!(!dict.has_prefix("zz"));
    }

    #[test]
    fn gaddag_dictionary_basic() {
        let dict = GaddagDictionary::from_words(vec!["CARE".to_string(), "CARES".to_string()], true);
        assert!(dict.contains("care"));
        assert!(dict.has_prefix("ca"));
    }

    #[test]
    fn gaddag_forms_for_cares() {
        // Validate GADDAG encoded forms exist in the internal trie
        let gd = GaddagDictionary::from_words(vec!["CARES".to_string()], true);
        // helper to check presence of a sequence in internal g_nodes
        fn has_seq(gd: &GaddagDictionary, s: &str) -> bool {
            let mut node = 0usize;
            for ch in s.chars() {
                if let Some(&nxt) = gd.g_nodes[node].edges.get(&ch) {
                    node = nxt;
                } else {
                    return false;
                }
            }
            gd.g_nodes[node].terminal
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
    fn gaddag_normalization_and_casefold() {
        // composed vs decomposed + casefold behavior
        let composed = "Café".to_string();
        let decomposed = "Cafe\u{301}".nfc().collect::<String>();
        let gd = GaddagDictionary::from_words(vec![decomposed.clone()], true);
        assert!(gd.contains(&composed));
        assert!(gd.has_prefix("caf"));
    }

    #[test]
    fn rules_with_fst_dictionary() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
                TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
            ],
        };
        let mut counts = HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        let cfg = GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout { width: 5, height: 5 },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 5,
            tile_counts: counts,
        };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(vec!["AB".to_string(), "B".to_string()], true)));
        let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
        // First move: single A invalid (not in dict)
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft { placements: vec![(c, Tile { kind_id: "A".into(), mark: None })] };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert!(sc1.main_score < 0);
        // Place AB horizontally: allowed
        let right = st.board.geom.to_cell_id(Coord2D { x: st.board.geom.from_cell_id(c).unwrap().x + 1, y: st.board.geom.from_cell_id(c).unwrap().y }).unwrap();
        let mv2 = MoveDraft { placements: vec![
            (c, Tile { kind_id: "A".into(), mark: None }),
            (right, Tile { kind_id: "B".into(), mark: None }),
        ] };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.total >= 0);
    }

    #[test]
    fn cross_word_validation_with_dict() {
        use std::collections::HashMap;
        let tileset = Tileset {
            tile_kinds: vec![
                TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
                TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
            ],
        };
        let mut counts = HashMap::new(); counts.insert("A".to_string(), 10); counts.insert("B".to_string(), 10);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 7, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        // dict only allows AB
        st.dictionary = Some(Box::new(FstDictionary::from_words(vec!["AB".to_string()], true)));
        let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
        // First move: place A at center
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft { placements: vec![(c, Tile { kind_id: "A".into(), mark: None })] };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert!(sc1.main_score < 0); // A alone not in dict
        // Still commit A to set up cross check
        let free_rules = CrosswordRules { free_word_mode: true, ..Default::default() };
        let v1b = free_rules.validate(&st, &mv1).unwrap();
        let sc1b = free_rules.score(&st, &v1b);
        free_rules.commit(&mut st, v1b, &sc1b).unwrap();
        // Second move: place B to the right to form main word "AB"
        let c_coord = st.board.geom.from_cell_id(c).unwrap();
        let right = st.board.geom.to_cell_id(Coord2D { x: c_coord.x + 1, y: c_coord.y }).unwrap();
        let mv2 = MoveDraft { placements: vec![(right, Tile { kind_id: "B".into(), mark: None })] };
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
        st.dictionary = Some(Box::new(SetDictionary::from_words(vec!["AB".to_string()], true)));
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
}
