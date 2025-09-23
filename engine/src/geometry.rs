use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

use crate::{serde_cell_adj, serde_cell_set, EngineError};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CellId(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    fn is_empty(&self) -> bool { self.len() == 0 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectGridGeometry {
    pub width: u32,
    pub height: u32,
    // Optional graph overlay: restrict present cells and override adjacency with direction tags
    #[serde(default, with = "serde_cell_adj")]
    adj: Option<HashMap<CellId, Vec<(CellId, String)>>>,
    #[serde(default, with = "serde_cell_set")]
    present: Option<HashSet<CellId>>,
}

impl RectGridGeometry {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, adj: None, present: None }
    }
    fn index(&self, c: Coord2D) -> Option<u32> {
        if c.x < 0 || c.y < 0 { return None; }
        let (x, y) = (c.x as u32, c.y as u32);
        if x < self.width && y < self.height { Some(y * self.width + x) } else { None }
    }

    pub fn has_graph(&self) -> bool { self.adj.is_some() }

    pub fn apply_graph_overlay(&mut self, overlay: GraphOverlay) -> Result<(), EngineError> {
        let mut present = HashSet::new();
        let mut id_for: Vec<CellId> = Vec::with_capacity(overlay.nodes.len());
        for c in overlay.nodes.iter() {
            if let Some(id) = self.index(*c).map(CellId) { present.insert(id); id_for.push(id); }
            else { return Err(EngineError::Config("overlay node outside bounds")); }
        }
        let mut adj: HashMap<CellId, Vec<(CellId, String)>> = HashMap::new();
        for (ai, bi, dir) in overlay.edges.into_iter() {
            if ai >= id_for.len() || bi >= id_for.len() {
                return Err(EngineError::Config("edge index out of range"));
            }
            let a = id_for[ai];
            let b = id_for[bi];
            if !present.contains(&a) || !present.contains(&b) {
                return Err(EngineError::Config("edge references missing node"));
            }
            adj.entry(a).or_default().push((b, dir.clone()));
            adj.entry(b).or_default().push((a, dir));
        }
        self.present = Some(present);
        self.adj = Some(adj);
        Ok(())
    }

    pub fn neighbors_with_tags(&self, id: CellId) -> SmallVec<[(CellId, &str); 8]> {
        let mut out: SmallVec<[(CellId, &str); 8]> = SmallVec::new();
        if let Some(adj) = &self.adj {
            if let Some(v) = adj.get(&id) { for (n, tag) in v { out.push((*n, tag.as_str())); } }
        } else if let Some(c) = self.from_cell_id(id) {
            let dirs = [
                (Coord2D { x: c.x - 1, y: c.y }, "W"),
                (Coord2D { x: c.x + 1, y: c.y }, "E"),
                (Coord2D { x: c.x, y: c.y - 1 }, "N"),
                (Coord2D { x: c.x, y: c.y + 1 }, "S"),
            ];
            for (d, tag) in dirs { if let Some(n) = self.to_cell_id(d) { out.push((n, tag)); } }
        }
        out
    }

    pub fn dir_tag_between(&self, a: CellId, b: CellId) -> Option<&str> {
        if let Some(adj) = &self.adj {
            if let Some(v) = adj.get(&a) { for (n, tag) in v { if *n == b { return Some(tag.as_str()); } } }
            None
        } else {
            let ac = self.from_cell_id(a)?;
            let bc = self.from_cell_id(b)?;
            if ac.x == bc.x {
                if ac.y + 1 == bc.y { Some("S") } else if ac.y - 1 == bc.y { Some("N") } else { None }
            } else if ac.y == bc.y {
                if ac.x + 1 == bc.x { Some("E") } else if ac.x - 1 == bc.x { Some("W") } else { None }
            } else { None }
        }
    }
}

impl BoardGeometry for RectGridGeometry {
    fn neighbors(&self, id: CellId) -> SmallVec<[CellId; 4]> {
        if let Some(adj) = &self.adj {
            let mut out: SmallVec<[CellId; 4]> = SmallVec::new();
            if let Some(v) = adj.get(&id) { for (n, _) in v { out.push(*n); } }
            out
        } else {
            let mut out: SmallVec<[CellId; 4]> = SmallVec::new();
            if let Some(c) = self.from_cell_id(id) {
                let dirs = [
                    Coord2D { x: c.x - 1, y: c.y },
                    Coord2D { x: c.x + 1, y: c.y },
                    Coord2D { x: c.x, y: c.y - 1 },
                    Coord2D { x: c.x, y: c.y + 1 },
                ];
                for d in dirs { if let Some(n) = self.to_cell_id(d) { out.push(n); } }
            }
            out
        }
    }

    fn to_cell_id(&self, c: Coord2D) -> Option<CellId> {
        let id = self.index(c).map(CellId)?;
        if let Some(p) = &self.present && !p.contains(&id) { return None; }
        Some(id)
    }

    fn from_cell_id(&self, id: CellId) -> Option<Coord2D> {
        let i = id.0;
        if i >= self.width * self.height { return None; }
        let y = i / self.width;
        let x = i % self.width;
        Some(Coord2D { x: x as i32, y: y as i32 })
    }

    fn len(&self) -> usize { (self.width * self.height) as usize }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphOverlay {
    pub nodes: Vec<Coord2D>,
    pub edges: Vec<(usize, usize, String)>,
}
