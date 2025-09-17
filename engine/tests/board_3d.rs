use engine as eng;
use engine::{BoardGeometry, Rules};

fn cfg_wh(w: u32, h: u32) -> eng::GameConfig {
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
    counts.insert("B".into(), 50);
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

fn apply_3d_overlay(st: &mut eng::GameState, w: i32, h: i32, d: i32) {
    let mut nodes = Vec::new();
    for z in 0..d {
        for y in 0..h {
            for x in 0..w {
                nodes.push(eng::Coord2D { x, y: y + z * h });
            }
        }
    }
    let index = |x: i32, y: i32, z: i32| -> usize { ((y + z * h) * w + x) as usize };
    let mut edges: Vec<(usize, usize, String)> = Vec::new();
    let mut try_edge = |x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32, tag: &str| {
        if x2 < 0 || x2 >= w || y2 < 0 || y2 >= h || z2 < 0 || z2 >= d {
            return;
        }
        edges.push((index(x1, y1, z1), index(x2, y2, z2), tag.to_string()));
    };
    for z in 0..d {
        for y in 0..h {
            for x in 0..w {
                try_edge(x, y, z, x + 1, y, z, "X");
                try_edge(x, y, z, x, y + 1, z, "Y");
                try_edge(x, y, z, x, y, z + 1, "Z");
            }
        }
    }
    st.apply_graph_overlay(eng::GraphOverlay { nodes, edges })
        .unwrap();
}

#[test]
fn validate_z_line_and_score() {
    let w = 3;
    let h = 3;
    let d = 2; // depth 2
    let mut st = eng::GameState::new(&cfg_wh(w, h * d as u32), 2).unwrap();
    apply_3d_overlay(&mut st, w as i32, h as i32, d as i32);
    let rules = eng::CrosswordRules::default();
    // Place at (1,1,0) and (1,1,1) along Z
    let id0 = st
        .board
        .geom
        .to_cell_id(eng::Coord2D { x: 1, y: 1 })
        .unwrap();
    let id1 = st
        .board
        .geom
        .to_cell_id(eng::Coord2D {
            x: 1,
            y: 1 + h as i32,
        })
        .unwrap();
    let mv = eng::MoveDraft {
        placements: vec![
            (
                id0,
                eng::Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            ),
            (
                id1,
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
fn bonus_applies_in_3d() {
    let w = 3;
    let h = 3;
    let d = 2;
    let mut st = eng::GameState::new(&cfg_wh(w, h * d as u32), 2).unwrap();
    apply_3d_overlay(&mut st, w as i32, h as i32, d as i32);
    // Set a double word on the second cell
    let id1 = st
        .board
        .geom
        .to_cell_id(eng::Coord2D {
            x: 1,
            y: 1 + h as i32,
        })
        .unwrap();
    st.board.bonuses.insert(
        id1,
        eng::Bonus {
            letter_mul: 1,
            word_mul: 2,
            tags: std::collections::BTreeSet::new(),
        },
    );
    let rules = eng::CrosswordRules::default();
    let id0 = st
        .board
        .geom
        .to_cell_id(eng::Coord2D { x: 1, y: 1 })
        .unwrap();
    let mv = eng::MoveDraft {
        placements: vec![
            (
                id0,
                eng::Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            ),
            (
                id1,
                eng::Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            ),
        ],
    };
    let v = rules.validate(&st, &mv).unwrap();
    let sc = rules.score(&st, &v);
    assert!(sc.total >= 2 * (1 + 3)); // 2x word multiplier
}
