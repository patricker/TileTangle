use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

use crate::{geometry::BoardGeometry, serde_cell_bonus, CellId, Tile};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bonus {
    pub letter_mul: i8, // default 1
    pub word_mul: i8,   // default 1
    pub tags: BTreeSet<String>,
}

impl Default for Bonus {
    fn default() -> Self { Self { letter_mul: 1, word_mul: 1, tags: BTreeSet::new() } }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub stack: Vec<Tile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "G: Serialize", deserialize = "G: Deserialize<'de>"))]
pub struct Board<G: BoardGeometry> {
    pub geom: G,
    pub cells: Vec<Cell>,
    #[serde(default, with = "serde_cell_bonus")]
    pub bonuses: HashMap<CellId, Bonus>,
}

impl<G: BoardGeometry> Board<G> {
    pub fn new(geom: G) -> Self {
        let len = geom.len();
        Self { geom, cells: vec![Cell::default(); len], bonuses: HashMap::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonus_default_is_identity() {
        let b = Bonus::default();
        assert_eq!(b.letter_mul, 1);
        assert_eq!(b.word_mul, 1);
        assert!(b.tags.is_empty());
    }
}
