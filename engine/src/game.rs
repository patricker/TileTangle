use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::{CellId, Rack, Tile, Tileset};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub rack: Rack,
    pub score: i32,
    /// Maximum rack capacity for this player (number of tiles the rack should hold)
    #[serde(default = "Player::default_rack_capacity")]
    pub rack_capacity: usize,
}

impl Default for Player {
    fn default() -> Self { Self { rack: Rack::default(), score: 0, rack_capacity: Self::default_rack_capacity() } }
}

impl Player { const fn default_rack_capacity() -> usize { 7 } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEventKind {
    Play { placements: Vec<(CellId, Tile)>, score: i32, total: i32 },
    Draw { tiles: Vec<String> },
    Exchange { give: Vec<String>, take: Vec<String> },
    Pass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent { 
    pub turn: u32,
    pub player: usize,
    pub kind: GameEventKind,
    pub position_hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectBoardLayout { pub width: u32, pub height: u32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveDraft { pub placements: Vec<(CellId, Tile)> }

// Split: GameState and its impl moved to a submodule.
mod state;
pub use state::GameState;
