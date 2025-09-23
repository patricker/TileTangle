//! TileTangle Engine — Core Model (Phase 1)

use rand::{Rng, SeedableRng, rngs::StdRng};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
#[cfg(test)]
use std::collections::BTreeSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation;

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
pub use rules::crossword::{ScoreBreakdown, Rules, ReadingDirection, StackScoring, CrosswordRules, ValidatedMove};

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

    // Collect all non-blank symbols from the tileset as an alphabet
    fn alphabet_symbols(state: &GameState) -> HashSet<String> {
        let mut set = HashSet::new();
        for tk in &state.tileset.tile_kinds {
            if !tk.is_blank {
                set.insert(tk.symbol.clone());
            }
        }
        set
    }

    // Walk along a tagged axis starting at `next` and away from `prev`, collecting fixed symbols
    fn collect_chain_symbols_from(
        state: &GameState,
        mut next: CellId,
        mut prev: CellId,
        tag: &str,
    ) -> Vec<String> {
        let mut out = Vec::new();
        loop {
            let cell = &state.board.cells[next.0 as usize];
            if let Some(tile) = cell.stack.last() {
                let (_score, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, tile);
                out.push(sym);
            } else {
                break;
            }
            let mut advanced = false;
            for (n, t) in state.board.geom.neighbors_with_tags(next) {
                if t == tag && n != prev {
                    prev = next;
                    next = n;
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                break;
            }
        }
        out
    }

    // Allowed symbols for placing on `cell` along a single perpendicular axis `ctag`.
    fn allowed_for_cell_on_axis(
        state: &GameState,
        cell: CellId,
        ctag: &str,
        alphabet: &HashSet<String>,
    ) -> HashSet<String> {
        // neighbors on this axis
        let mut sides: Vec<CellId> = Vec::with_capacity(2);
        for (n, t) in state.board.geom.neighbors_with_tags(cell) {
            if t == ctag {
                sides.push(n);
            }
        }
        // gather fixed chains
        let mut left_syms: Vec<String> = Vec::new();
        let mut right_syms: Vec<String> = Vec::new();
        match (sides.get(0), sides.get(1)) {
            (None, None) => return alphabet.clone(),
            (Some(&a), None) | (None, Some(&a)) => {
                right_syms = collect_chain_symbols_from(state, a, cell, ctag);
                if right_syms.is_empty() {
                    return alphabet.clone();
                }
            }
            (Some(&a), Some(&b)) => {
                left_syms = collect_chain_symbols_from(state, a, cell, ctag);
                right_syms = collect_chain_symbols_from(state, b, cell, ctag);
                if left_syms.is_empty() && right_syms.is_empty() {
                    return alphabet.clone();
                }
            }
        }
        // If no dictionary, be permissive
        let Some(dict) = state.dictionary.as_ref() else {
            return alphabet.clone();
        };
        let mut allowed: HashSet<String> = HashSet::new();
        let base_tiles = left_syms.len() + right_syms.len();
        for sym in alphabet {
            // word in one reading
            let mut a = String::new();
            for s in left_syms.iter().rev() {
                a.push_str(s);
            }
            a.push_str(sym);
            for s in &right_syms {
                a.push_str(s);
            }
            let mut ok = base_tiles + 1 > 1 && dict.contains(&a);
            if !ok {
                // opposite reading
                let mut b = String::new();
                for s in right_syms.iter().rev() {
                    b.push_str(s);
                }
                b.push_str(sym);
                for s in &left_syms {
                    b.push_str(s);
                }
                ok = base_tiles + 1 > 1 && dict.contains(&b);
            }
            if ok || base_tiles == 0 {
                allowed.insert(sym.clone());
            }
        }
        allowed
    }

    // For a main-axis `tag`, compute allowed symbols per empty cell by intersecting across all
    // perpendicular tags present at the cell.
    fn cross_checks_for_tag(
        state: &GameState,
        tag: &str,
        alphabet: &HashSet<String>,
    ) -> HashMap<CellId, HashSet<String>> {
        let mut map: HashMap<CellId, HashSet<String>> = HashMap::new();
        for (idx, cell) in state.board.cells.iter().enumerate() {
            if !cell.stack.is_empty() {
                continue;
            }
            let cid = CellId(idx as u32);
            // collect distinct perpendicular tags
            let mut perp: HashSet<&str> = HashSet::new();
            for (_n, t) in state.board.geom.neighbors_with_tags(cid) {
                if t != tag {
                    perp.insert(t);
                }
            }
            let mut allowed = alphabet.clone();
            for ctag in perp {
                let on_axis = allowed_for_cell_on_axis(state, cid, ctag, alphabet);
                allowed = allowed
                    .into_iter()
                    .filter(|s| on_axis.contains(s))
                    .collect();
                if allowed.is_empty() {
                    break;
                }
            }
            map.insert(cid, allowed);
        }
        map
    }

    struct ExploreCtx<'a> {
        state: &'a GameState,
        rules: &'a dyn Rules,
        tile_symbols: &'a [(String, String)],
        blank_ids: &'a [String],
        cross: &'a HashMap<CellId, HashSet<String>>, // allowed symbols per cell on this axis
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

        // Cross-check filtered candidates at this cell
        let allowed_syms = ctx.cross.get(&cid);

        if let Some(allowed) = allowed_syms {
            // Try normal tiles that yield allowed symbols
            for (kind_id, symbol) in ctx.tile_symbols.iter() {
                if !allowed.contains(symbol) {
                    continue;
                }
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

            // Try blank tiles assigning only allowed symbols
            for blank_id in ctx.blank_ids {
                let available = rack_counts.get(blank_id).copied().unwrap_or(0);
                if available == 0 {
                    continue;
                }
                {
                    let entry = rack_counts.get_mut(blank_id).unwrap();
                    *entry -= 1;
                }
                for symbol in allowed {
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
        cross: &HashMap::new(), // replaced per tag below
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
            // Precompute cross-checks for this axis tag
            let alphabet = alphabet_symbols(state);
            let cross_checks = cross_checks_for_tag(state, &tag, &alphabet);
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
                    let mut tag_ctx = ExploreCtx {
                        state: ctx.state,
                        rules: ctx.rules,
                        tile_symbols: ctx.tile_symbols,
                        blank_ids: ctx.blank_ids,
                        cross: &cross_checks,
                        seen: ctx.seen,
                        out: ctx.out,
                    };
                    explore_segment(segment, 0, &mut rack_counts, &mut placements, &mut tag_ctx);
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
}
