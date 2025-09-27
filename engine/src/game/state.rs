use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::dict::Dictionary;
use crate::text::nfc;
use crate::{
    EngineError, GameConfig, GameEvent, GameEventKind, Player, PlayerId, Tile, TileKind,
    board::Board,
    geometry::RectGridGeometry,
    inventory::{Bag, Tileset},
};

// Zobrist hashing helpers used by GameState
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn zobrist_mix(seed: u64, value: impl Hash) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    value.hash(&mut hasher);
    splitmix64(hasher.finish())
}

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
    pub fn preview(
        &self,
        draft: &crate::game::MoveDraft,
    ) -> Result<Board<RectGridGeometry>, EngineError> {
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
    pub fn apply_graph_overlay(
        &mut self,
        overlay: crate::geometry::GraphOverlay,
    ) -> Result<(), EngineError> {
        self.board.geom.apply_graph_overlay(overlay)
    }

    pub(crate) fn push_event(&mut self, player: usize, kind: GameEventKind) {
        let hash = self.compute_position_hash();
        let turn = self.turn_num;
        self.event_log.push(GameEvent {
            turn,
            player,
            kind,
            position_hash: hash,
        });
    }

    pub(crate) fn advance_turn(&mut self) {
        let pid = self.to_move.0;
        self.turn_num = self.turn_num.saturating_add(1);
        if !self.players.is_empty() {
            self.to_move = PlayerId((pid + 1) % self.players.len());
        }
    }

    pub(crate) fn log_draw(&mut self, player: usize, tiles: &[Tile]) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Tile, TileKind,
        game::{GameConfig, MoveDraft, RectBoardLayout},
        geometry::{BoardGeometry, Coord2D},
        inventory::Tileset,
    };
    use std::collections::HashMap;

    #[test]
    fn state_new_and_preview() {
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
}
