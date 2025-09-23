use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{serde_tile_counts, EngineError, Tile, TileKind};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
        if i < self.tiles.len() { Some(self.tiles.remove(i)) } else { None }
    }
    pub fn len(&self) -> usize { self.tiles.len() }
    pub fn is_empty(&self) -> bool { self.tiles.is_empty() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tileset {
    pub tile_kinds: Vec<TileKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bag {
    #[serde(with = "serde_tile_counts")]
    pub counts: HashMap<TileKind, u32>,
    rng: ChaCha12Rng,
}

impl Bag {
    pub fn from_tileset(ts: &Tileset, seed: u64) -> Self {
        let mut counts = HashMap::new();
        for tk in &ts.tile_kinds {
            counts.insert(tk.clone(), 0);
        }
        Self { counts, rng: ChaCha12Rng::seed_from_u64(seed) }
    }

    pub fn with_counts(counts: HashMap<TileKind, u32>, seed: u64) -> Self {
        Self { counts, rng: ChaCha12Rng::seed_from_u64(seed) }
    }

    pub fn remaining(&self) -> u32 { self.counts.values().copied().sum() }

    pub fn draw(&mut self, n: usize) -> Vec<Tile> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n { if let Some(t) = self.draw_one() { out.push(t); } }
        out
    }

    pub fn draw_one(&mut self) -> Option<Tile> {
        let total = self.remaining();
        if total == 0 { return None; }
        let choice = self.rng.gen_range(0..total);
        let mut items: Vec<(&TileKind, &u32)> = self.counts.iter().filter(|(_, c)| **c > 0).collect();
        items.sort_by(|(k1, _), (k2, _)| k1.id.cmp(&k2.id));
        let mut acc = 0u32;
        let mut selected_key: Option<TileKind> = None;
        for (k, c) in items {
            acc += *c;
            if choice < acc { selected_key = Some(k.clone()); break; }
        }
        if let Some(key) = selected_key { let cnt = self.counts.get_mut(&key).unwrap(); *cnt -= 1; Some(Tile { kind_id: key.id.clone(), mark: None }) } else { None }
    }
}

