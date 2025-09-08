//! TileTangle Engine — Core Model (Phase 1)

use rand::{Rng, SeedableRng, rngs::StdRng};
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

#[derive(Debug, Clone)]
pub struct GameState {
    pub board: Board<RectGridGeometry>,
    pub players: Vec<Player>,
    pub to_move: PlayerId,
    pub bag: Bag,
    pub turn_num: u32,
    pub tileset: Tileset,
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

impl Player {
    fn rack_size(&self) -> Option<usize> {
        Some(7)
    }
}

// -------- Tests --------

#[cfg(test)]
mod tests {
    use super::*;

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
}
