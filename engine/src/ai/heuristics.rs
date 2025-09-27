use std::collections::HashMap;

use crate::geometry::BoardGeometry;
use crate::{GameState, Tile, Tileset, geometry::CellId, movegen::CandidateMove};

use super::AiConfig;
use super::types::EvaluatedMove;

fn tileset_symbol_for_kind(tileset: &Tileset, kind_id: &str) -> String {
    tileset
        .tile_kinds
        .iter()
        .find(|tk| tk.id == kind_id)
        .map(|tk| tk.symbol.clone())
        .unwrap_or_else(|| kind_id.to_string())
}

pub(crate) fn leftover_counts_from_rack(
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

pub fn rack_leave_score(
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

pub fn board_equity_bonus(state: &GameState, candidate: &CandidateMove) -> i32 {
    use std::collections::HashSet;
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

pub fn endgame_penalty(state: &GameState, leftover: &HashMap<String, usize>) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameConfig, RectBoardLayout, TileKind, Tileset};

    fn basic_config() -> GameConfig {
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
                TileKind {
                    id: "Q".into(),
                    symbol: "Q".into(),
                    score: 10,
                    is_blank: false,
                    aliases: vec![],
                },
                TileKind {
                    id: "?".into(),
                    symbol: "?".into(),
                    score: 0,
                    is_blank: true,
                    aliases: vec![],
                },
            ],
        };
        let mut counts = std::collections::HashMap::new();
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        counts.insert("Q".to_string(), 1);
        counts.insert("?".to_string(), 2);
        GameConfig {
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
        }
    }

    #[test]
    fn leftover_counts_basic() {
        let placements = vec![];
        let rack = vec!["A".to_string(), "B".to_string(), "A".to_string()];
        let counts = leftover_counts_from_rack(&rack, &placements);
        assert_eq!(counts.get("A"), Some(&2));
        assert_eq!(counts.get("B"), Some(&1));
        assert!(!counts.contains_key("Q"));
    }

    #[test]
    fn rack_leave_looks_up_symbol_then_id() {
        let cfg = crate::ai::AiConfig::default();
        let tileset = basic_config().tileset;
        let mut leftover = HashMap::new();
        leftover.insert("A".to_string(), 2);
        let score = rack_leave_score(&cfg, &tileset, &leftover);
        // default table gives A => +1 each
        assert_eq!(score, 2);
    }

    #[test]
    fn endgame_penalty_applies_when_bag_empty() {
        let cfg = basic_config();
        let mut st = crate::GameState::new(&cfg, 1).unwrap();
        for v in st.bag.counts.values_mut() {
            *v = 0;
        }
        let mut leftover = HashMap::new();
        leftover.insert("Q".to_string(), 1);
        let pen = endgame_penalty(&st, &leftover);
        assert_eq!(pen, -10);
    }

    #[test]
    fn board_equity_counts_open_neighbors() {
        let cfg = basic_config();
        let st = crate::GameState::new(&cfg, 1).unwrap();
        // Place a single tile at center as candidate; empty board otherwise.
        let center = crate::CrosswordRules::center_cell(&st.board.geom);
        let candidate = crate::movegen::CandidateMove {
            placements: vec![(
                center,
                crate::Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            )],
            word: "A".into(),
            score: 0,
        };
        let eq = board_equity_bonus(&st, &candidate);
        // In a 5x5 grid, center has 4 orthogonal neighbors.
        assert_eq!(eq, 4);
    }
}
