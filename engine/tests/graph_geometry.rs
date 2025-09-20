use engine as eng;
use engine::BoardGeometry;
use engine::Rules;

fn make_cfg(w: u32, h: u32) -> eng::GameConfig {
    let tileset = eng::Tileset {
        tile_kinds: vec![
            eng::TileKind {
                id: "A".into(),
                symbol: "A".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            },
            eng::TileKind {
                id: "B".into(),
                symbol: "B".into(),
                score: 3,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert("A".into(), 50);
    counts.insert("B".into(), 20);
    eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: w,
            height: h,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 1,
        tile_counts: counts,
    }
}

#[test]
fn hex_neighbors_counts() {
    let cfg = make_cfg(7, 7);
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    // Build even-r hex overlay
    let mut nodes = Vec::new();
    for y in 0..7i32 {
        for x in 0..7i32 {
            nodes.push(eng::Coord2D { x, y });
        }
    }
    let idx = |x: i32, y: i32| (y * 7 + x) as usize;
    let mut edges: Vec<(usize, usize, String)> = Vec::new();
    let try_edge =
        |edges: &mut Vec<(usize, usize, String)>, x1: i32, y1: i32, x2: i32, y2: i32, dir: &str| {
            if !(0..7).contains(&x2) || !(0..7).contains(&y2) {
                return;
            }
            edges.push((idx(x1, y1), idx(x2, y2), dir.to_string()));
        };
    for y in 0..7i32 {
        for x in 0..7i32 {
            let even = (y % 2) == 0;
            try_edge(&mut edges, x, y, x + 1, y, "E");
            try_edge(&mut edges, x, y, x + if even { 0 } else { 1 }, y - 1, "NE");
            try_edge(&mut edges, x, y, x + if even { 0 } else { 1 }, y + 1, "SE");
        }
    }
    st.apply_graph_overlay(eng::GraphOverlay { nodes, edges })
        .unwrap();
    // center should have 6 neighbors
    let c = st
        .board
        .geom
        .to_cell_id(eng::Coord2D { x: 3, y: 3 })
        .unwrap();
    assert_eq!(st.board.geom.neighbors(c).len(), 6);
}

#[test]
fn graph_line_and_contiguity() {
    let cfg = make_cfg(5, 5);
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    // Build simple straight line along row y=2 with tag "E"
    let nodes = (0..5).map(|x| eng::Coord2D { x, y: 2 }).collect::<Vec<_>>();
    let mut edges = Vec::new();
    for x in 0..4usize {
        edges.push((x, x + 1, "E".into()));
    }
    st.apply_graph_overlay(eng::GraphOverlay { nodes, edges })
        .unwrap();
    let rules = eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    // Place at (1,2) and (3,2) with an existing tile at (2,2) bridging the gap
    let id2 = st
        .board
        .geom
        .to_cell_id(eng::Coord2D { x: 2, y: 2 })
        .unwrap();
    st.board.cells[id2.0 as usize].stack.push(eng::Tile {
        kind_id: "A".into(),
        mark: None,
    });
    let mv = eng::MoveDraft {
        placements: vec![
            (
                st.board
                    .geom
                    .to_cell_id(eng::Coord2D { x: 1, y: 2 })
                    .unwrap(),
                eng::Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            ),
            (
                st.board
                    .geom
                    .to_cell_id(eng::Coord2D { x: 3, y: 2 })
                    .unwrap(),
                eng::Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            ),
        ],
    };
    let v = rules.validate(&st, &mv).unwrap();
    let sc = rules.score(&st, &v);
    assert!(sc.total >= 0);
}

#[test]
fn playground_diamond_overlay_is_valid() {
    let cfg = make_cfg(9, 9);
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    let nodes = vec![
        eng::Coord2D { x: 4, y: 0 },
        eng::Coord2D { x: 3, y: 1 },
        eng::Coord2D { x: 4, y: 1 },
        eng::Coord2D { x: 5, y: 1 },
        eng::Coord2D { x: 2, y: 2 },
        eng::Coord2D { x: 3, y: 2 },
        eng::Coord2D { x: 4, y: 2 },
        eng::Coord2D { x: 5, y: 2 },
        eng::Coord2D { x: 6, y: 2 },
        eng::Coord2D { x: 1, y: 3 },
        eng::Coord2D { x: 2, y: 3 },
        eng::Coord2D { x: 3, y: 3 },
        eng::Coord2D { x: 4, y: 3 },
        eng::Coord2D { x: 5, y: 3 },
        eng::Coord2D { x: 6, y: 3 },
        eng::Coord2D { x: 7, y: 3 },
        eng::Coord2D { x: 0, y: 4 },
        eng::Coord2D { x: 1, y: 4 },
        eng::Coord2D { x: 2, y: 4 },
        eng::Coord2D { x: 3, y: 4 },
        eng::Coord2D { x: 4, y: 4 },
        eng::Coord2D { x: 5, y: 4 },
        eng::Coord2D { x: 6, y: 4 },
        eng::Coord2D { x: 7, y: 4 },
        eng::Coord2D { x: 8, y: 4 },
        eng::Coord2D { x: 1, y: 5 },
        eng::Coord2D { x: 2, y: 5 },
        eng::Coord2D { x: 3, y: 5 },
        eng::Coord2D { x: 4, y: 5 },
        eng::Coord2D { x: 5, y: 5 },
        eng::Coord2D { x: 6, y: 5 },
        eng::Coord2D { x: 7, y: 5 },
        eng::Coord2D { x: 2, y: 6 },
        eng::Coord2D { x: 3, y: 6 },
        eng::Coord2D { x: 4, y: 6 },
        eng::Coord2D { x: 5, y: 6 },
        eng::Coord2D { x: 6, y: 6 },
        eng::Coord2D { x: 3, y: 7 },
        eng::Coord2D { x: 4, y: 7 },
        eng::Coord2D { x: 5, y: 7 },
        eng::Coord2D { x: 4, y: 8 },
    ];
    let edges = vec![
        (0, 2, "S"),
        (1, 2, "E"),
        (1, 5, "S"),
        (2, 3, "E"),
        (2, 1, "W"),
        (2, 0, "N"),
        (2, 6, "S"),
        (3, 2, "W"),
        (3, 7, "S"),
        (4, 5, "E"),
        (4, 10, "S"),
        (5, 6, "E"),
        (5, 4, "W"),
        (5, 1, "N"),
        (5, 11, "S"),
        (6, 7, "E"),
        (6, 5, "W"),
        (6, 2, "N"),
        (6, 12, "S"),
        (7, 8, "E"),
        (7, 6, "W"),
        (7, 3, "N"),
        (7, 13, "S"),
        (8, 7, "W"),
        (8, 14, "S"),
        (9, 10, "E"),
        (9, 17, "S"),
        (10, 11, "E"),
        (10, 9, "W"),
        (10, 4, "N"),
        (10, 18, "S"),
        (11, 12, "E"),
        (11, 10, "W"),
        (11, 5, "N"),
        (11, 19, "S"),
        (12, 13, "E"),
        (12, 11, "W"),
        (12, 6, "N"),
        (12, 20, "S"),
        (13, 14, "E"),
        (13, 12, "W"),
        (13, 7, "N"),
        (13, 21, "S"),
        (14, 15, "E"),
        (14, 13, "W"),
        (14, 8, "N"),
        (14, 22, "S"),
        (15, 14, "W"),
        (15, 23, "S"),
        (16, 17, "E"),
        (17, 18, "E"),
        (17, 16, "W"),
        (17, 9, "N"),
        (17, 25, "S"),
        (18, 19, "E"),
        (18, 17, "W"),
        (18, 10, "N"),
        (18, 26, "S"),
        (19, 20, "E"),
        (19, 18, "W"),
        (19, 11, "N"),
        (19, 27, "S"),
        (20, 21, "E"),
        (20, 19, "W"),
        (20, 12, "N"),
        (20, 28, "S"),
        (21, 22, "E"),
        (21, 20, "W"),
        (21, 13, "N"),
        (21, 29, "S"),
        (22, 23, "E"),
        (22, 21, "W"),
        (22, 14, "N"),
        (22, 30, "S"),
        (23, 24, "E"),
        (23, 22, "W"),
        (23, 15, "N"),
        (23, 31, "S"),
        (24, 23, "W"),
        (25, 26, "E"),
        (25, 17, "N"),
        (26, 27, "E"),
        (26, 25, "W"),
        (26, 18, "N"),
        (26, 32, "S"),
        (27, 28, "E"),
        (27, 26, "W"),
        (27, 19, "N"),
        (27, 33, "S"),
        (28, 29, "E"),
        (28, 27, "W"),
        (28, 20, "N"),
        (28, 34, "S"),
        (29, 30, "E"),
        (29, 28, "W"),
        (29, 21, "N"),
        (29, 35, "S"),
        (30, 31, "E"),
        (30, 29, "W"),
        (30, 22, "N"),
        (30, 36, "S"),
        (31, 30, "W"),
        (31, 23, "N"),
        (32, 33, "E"),
        (32, 26, "N"),
        (33, 34, "E"),
        (33, 32, "W"),
        (33, 27, "N"),
        (33, 37, "S"),
        (34, 35, "E"),
        (34, 33, "W"),
        (34, 28, "N"),
        (34, 38, "S"),
        (35, 36, "E"),
        (35, 34, "W"),
        (35, 29, "N"),
        (35, 39, "S"),
        (36, 35, "W"),
        (36, 30, "N"),
        (37, 38, "E"),
        (37, 33, "N"),
        (38, 39, "E"),
        (38, 37, "W"),
        (38, 34, "N"),
        (38, 40, "S"),
        (39, 38, "W"),
        (39, 35, "N"),
        (40, 38, "N"),
    ];
    let edges = edges
        .into_iter()
        .map(|(a, b, dir)| (a, b, dir.to_string()))
        .collect();
    st.apply_graph_overlay(eng::GraphOverlay { nodes, edges })
        .unwrap();

    // Rules are not needed for this rack sizing assertion
    for pid in 0..st.players.len() {
        loop {
            if st.players[pid].rack.tiles.len() >= cfg.rack_size {
                break;
            }
            if let Some(tile) = st.bag.draw_one() {
                let _ = st.players[pid].rack.add(tile, cfg.rack_size);
            } else {
                break;
            }
        }
    }

    assert_eq!(st.players[0].rack.tiles.len(), cfg.rack_size.min(7));
}
