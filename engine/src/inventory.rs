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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rack_add_remove_capacity() {
        let mut r = Rack::default();
        let rs = 2;
        r.add(Tile { kind_id: "A".into(), mark: None }, rs).unwrap();
        r.add(Tile { kind_id: "B".into(), mark: None }, rs).unwrap();
        assert!(matches!(r.add(Tile { kind_id: "C".into(), mark: None }, rs), Err(EngineError::RackCapacity)));
        assert_eq!(r.len(), 2);
        let t = r.remove_at(0).unwrap();
        assert_eq!(t.kind_id, "A");
        assert_eq!(r.len(), 1);
    }

    fn make_counts() -> HashMap<TileKind, u32> {
        let a = TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] };
        let b = TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] };
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
}
