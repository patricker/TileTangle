//! TileTangle Engine — Core Model

use serde::{Deserialize, Serialize};
//
use std::hash::{Hash, Hasher};
//

mod serde_cell_adj;
mod serde_cell_bonus;
mod serde_cell_set;
mod serde_tile_counts;

// -------- Error, Version, Text submodules --------
pub mod error;
pub use error::EngineError;

pub mod version;
pub use version::{EngineVersion, engine_version};

pub mod text;
pub use text::{NormalizationMode, Symbol, Tokenizer, TokenizerRef, nfc, normalize_with_mode};

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
pub use game::{
    GameConfig, GameEvent, GameEventKind, GameState, MoveDraft, Player, PlayerId, RectBoardLayout,
};

// -------- Rules (plugins extracted) --------
pub mod rules;
pub use rules::crossword::{
    CrosswordRules, ReadingDirection, Rules, ScoreBreakdown, StackScoring, ValidatedMove,
};
pub use rules::plugins::{
    Action, BasicActionsPlugin, PluginRules, RulePlugin, ScoreBonusPlugin, UserMove,
};

// -------- Inventory (module) --------
pub mod inventory;
pub use inventory::{Bag, Rack, Tileset};

// -------- Move generation --------
pub mod movegen;
pub use movegen::{CandidateMove, generate_moves};

// -------- AI --------
pub mod ai;
pub use ai::{
    AiConfig, AiDifficulty, AiStrategy, EvaluatedMove, OpponentModel, best_move, best_move_greedy,
    evaluate_candidate_move,
};

impl Player {
    fn rack_size(&self) -> Option<usize> {
        Some(self.rack_capacity.max(1))
    }
}

// -------- Dictionary Engine (Phase 3) --------
// Moved to module `dict`; re-exported here for compatibility
pub mod dict;
pub use dict::{
    DawgDictionary, Dictionary, DictionaryOptions, FstDictionary, GaddagCursor, GaddagDictionary,
    GaddagRight, SetDictionary,
};

// -------- Tests --------

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_smoke() {
        assert_eq!(engine_version().major, 0);
    }
}
