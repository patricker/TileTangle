use engine::{generate_moves, AiDifficulty, BoardGeometry, CrosswordRules, GameConfig, GameState, GraphOverlay, RectBoardLayout, Tile, TileKind, Tileset, best_move};

fn hex_state_with_center_tile() -> GameState {
    let tileset = Tileset {
        tile_kinds: vec![
            TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
            TileKind { id: "C".into(), symbol: "C".into(), score: 3, is_blank: false, aliases: vec![] },
        ],
    };
    let cfg = GameConfig {
        tileset,
        rack_size: 7,
        board_layout: RectBoardLayout { width: 5, height: 5 },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 1,
        tile_counts: vec![
            ("A".into(), 30),
            ("B".into(), 30),
            ("C".into(), 30),
        ]
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

#[test]
fn graph_generate_moves_returns_candidates() {
    let state = hex_state_with_center_tile();
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let moves = generate_moves(
        &state,
        &rules,
        &vec!["B".into(), "C".into(), "A".into()],
        7,
    );
    assert!(!moves.is_empty(), "expected at least one move for graph board");
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
    assert!(best.is_some(), "expected non-empty best move for graph board");
}
