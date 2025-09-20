use engine::{
    AiDifficulty, BoardGeometry, Coord2D, CrosswordRules, GameConfig, GameState, GraphOverlay,
    RectBoardLayout, Tile, TileKind, Tileset, best_move, generate_moves,
};
use std::collections::HashMap;

fn hex_state_with_center_tile() -> GameState {
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
                id: "C".into(),
                symbol: "C".into(),
                score: 3,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
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
        tile_counts: vec![("A".into(), 30), ("B".into(), 30), ("C".into(), 30)]
            .into_iter()
            .collect(),
    };
    let mut state = GameState::new(&cfg, 2).unwrap();
    let width = 5_i32;
    let height = 5_i32;
    let mut nodes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            nodes.push(engine::Coord2D { x, y });
        }
    }
    let idx = |x: i32, y: i32| -> usize { (y * width + x) as usize };
    let mut edges = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let even = y % 2 == 0;
            let east_shift = if even { 0 } else { 1 };
            let west_shift = if even { -1 } else { 0 };
            let mut add = |x1: i32, y1: i32, x2: i32, y2: i32, dir: &str| {
                if x2 < 0 || x2 >= width || y2 < 0 || y2 >= height {
                    return;
                }
                edges.push((idx(x1, y1), idx(x2, y2), dir.to_string()));
            };
            add(x, y, x + 1, y, "E");
            add(x, y, x - 1, y, "W");
            add(x, y, x + east_shift, y - 1, "NE");
            add(x, y, x + west_shift, y - 1, "NW");
            add(x, y, x + east_shift, y + 1, "SE");
            add(x, y, x + west_shift, y + 1, "SW");
        }
    }
    state
        .apply_graph_overlay(GraphOverlay { nodes, edges })
        .unwrap();
    // Seed board with an existing tile at the center to create an anchor.
    let center = state
        .board
        .geom
        .to_cell_id(engine::Coord2D { x: 2, y: 2 })
        .unwrap();
    state.board.cells[center.0 as usize].stack.push(Tile {
        kind_id: "A".into(),
        mark: None,
    });
    state
}

fn overlay_from_mask<F>(width: i32, height: i32, is_active: F) -> GraphOverlay
where
    F: Fn(i32, i32) -> bool,
{
    let mut nodes = Vec::new();
    let mut index = HashMap::new();
    for y in 0..height {
        for x in 0..width {
            if is_active(x, y) {
                let idx = nodes.len();
                nodes.push(Coord2D { x, y });
                index.insert((x, y), idx);
            }
        }
    }

    let mut edges = Vec::new();
    let directions = [(1, 0, "E"), (-1, 0, "W"), (0, -1, "N"), (0, 1, "S")];

    for (&(x, y), &from_idx) in &index {
        for (dx, dy, dir) in directions {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || nx >= width || ny < 0 || ny >= height {
                continue;
            }
            if let Some(&to_idx) = index.get(&(nx, ny)) {
                edges.push((from_idx, to_idx, dir.to_string()));
            }
        }
    }

    GraphOverlay { nodes, edges }
}

fn graph_state_with_mask<F>(width: i32, height: i32, is_active: F, anchor: Coord2D) -> GameState
where
    F: Fn(i32, i32) -> bool,
{
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
                id: "C".into(),
                symbol: "C".into(),
                score: 3,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let cfg = GameConfig {
        tileset,
        rack_size: 7,
        board_layout: RectBoardLayout {
            width: width as u32,
            height: height as u32,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 1,
        tile_counts: vec![("A".into(), 30), ("B".into(), 30), ("C".into(), 30)]
            .into_iter()
            .collect(),
    };
    let mut state = GameState::new(&cfg, 2).unwrap();
    let overlay = overlay_from_mask(width, height, is_active);
    state.apply_graph_overlay(overlay).unwrap();
    let anchor_id = state
        .board
        .geom
        .to_cell_id(anchor)
        .expect("anchor must be active in overlay");
    state.board.cells[anchor_id.0 as usize].stack.push(Tile {
        kind_id: "A".into(),
        mark: None,
    });
    state
}

#[test]
fn graph_generate_moves_returns_candidates() {
    let state = hex_state_with_center_tile();
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let moves = generate_moves(&state, &rules, &["B".into(), "C".into(), "A".into()], 7);
    assert!(
        !moves.is_empty(),
        "expected at least one move for graph board"
    );
}

#[test]
fn triangle_graph_generate_moves_returns_candidates() {
    let state = graph_state_with_mask(5, 5, |x, y| x >= y, Coord2D { x: 2, y: 2 });
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let rack = vec!["B".into(), "C".into(), "A".into()];
    let moves = generate_moves(&state, &rules, &rack, 7);
    assert!(
        !moves.is_empty(),
        "expected at least one move for triangular graph board"
    );
    // Ensure the best-move path still succeeds. Give the player a rack that can play.
    let mut state2 = state.clone();
    state2.players[0].rack.tiles.clear();
    state2.players[0].rack.tiles.extend([
        Tile {
            kind_id: "B".into(),
            mark: None,
        },
        Tile {
            kind_id: "C".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
    ]);
    let best = best_move(&state2, &rules, AiDifficulty::Easy);
    assert!(
        best.is_some(),
        "expected best move for triangular graph board"
    );
}

#[test]
fn ring_graph_generate_moves_returns_candidates() {
    let width = 5;
    let height = 5;
    let ring_mask = |x: i32, y: i32| {
        let edge = x.min(width - 1 - x).min(y.min(height - 1 - y));
        edge == 0
    };
    let state = graph_state_with_mask(width, height, ring_mask, Coord2D { x: 0, y: 0 });
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let moves = generate_moves(&state, &rules, &["B".into(), "C".into(), "A".into()], 7);
    assert!(
        !moves.is_empty(),
        "expected at least one move for ring graph board"
    );
}

#[test]
fn ring_graph_best_move_produces_result() {
    let width = 5;
    let height = 5;
    let ring_mask = |x: i32, y: i32| {
        let edge = x.min(width - 1 - x).min(y.min(height - 1 - y));
        edge == 0
    };
    let mut state = graph_state_with_mask(width, height, ring_mask, Coord2D { x: 0, y: 0 });
    state.players[0].rack.tiles.clear();
    state.players[0].rack.tiles.extend([
        Tile {
            kind_id: "B".into(),
            mark: None,
        },
        Tile {
            kind_id: "C".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
    ]);

    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let best = best_move(&state, &rules, AiDifficulty::Easy);
    assert!(best.is_some(), "expected best move for ring graph board");
}

#[test]
fn graph_best_move_produces_result() {
    let mut state = hex_state_with_center_tile();
    // Give the active player a rack that can play along the hex axis.
    state.players[0].rack.tiles.clear();
    state.players[0].rack.tiles.extend([
        Tile {
            kind_id: "B".into(),
            mark: None,
        },
        Tile {
            kind_id: "C".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
    ]);
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let best = best_move(&state, &rules, AiDifficulty::Easy);
    assert!(
        best.is_some(),
        "expected non-empty best move for graph board"
    );
}
