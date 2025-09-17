use engine::{self as eng, BoardGeometry, Rules};

#[test]
fn generate_simple_horizontal_move() {
    // Tiles: A(1), B(3); dict allows AB only
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
    counts.insert("A".to_string(), 10);
    counts.insert("B".to_string(), 10);
    let cfg = eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: 5,
            height: 5,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 1,
        tile_counts: counts,
    };
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    st.dictionary = Some(Box::new(eng::FstDictionary::from_words(
        vec!["AB".to_string(), "B".to_string()],
        true,
    )));
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };
    // Seed board with A at center (free commit)
    let center = eng::CrosswordRules::center_cell(&st.board.geom);
    let mv = eng::MoveDraft {
        placements: vec![(
            center,
            eng::Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let fv = eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    }
    .validate(&st, &mv)
    .unwrap();
    let fs = eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    }
    .score(&st, &fv);
    eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    }
    .commit(&mut st, fv, &fs)
    .unwrap();
    // Rack has B
    let rack = vec!["B".to_string()];
    let moves = eng::generate_moves(&st, &rules, &rack, 7);
    assert!(
        moves
            .iter()
            .any(|cm| cm.word == "AB" && cm.placements.len() == 1)
    );
}

#[test]
fn generate_uses_blank_for_letter() {
    // Tiles: Blank(0), B(3); dict allows AB only
    let tileset = eng::Tileset {
        tile_kinds: vec![
            eng::TileKind {
                id: "BL".into(),
                symbol: "_".into(),
                score: 0,
                is_blank: true,
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
    counts.insert("BL".to_string(), 10);
    counts.insert("B".to_string(), 10);
    let cfg = eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: 5,
            height: 5,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 1,
        tile_counts: counts,
    };
    let st0 = eng::GameState::new(&cfg, 2).unwrap();
    let mut st = st0;
    st.dictionary = Some(Box::new(eng::FstDictionary::from_words(
        vec!["AB".to_string()],
        true,
    )));
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };
    // Empty board; rack has Blank and B
    let rack = vec!["BL".to_string(), "B".to_string()];
    let moves = eng::generate_moves(&st, &rules, &rack, 7);
    assert!(
        moves
            .iter()
            .any(|cm| cm.word == "AB" && cm.placements.len() == 2)
    );
}

#[test]
fn generate_with_multichar_tile() {
    // Tiles: QU(10), A(1); dict allows QUA
    let tileset = eng::Tileset {
        tile_kinds: vec![
            eng::TileKind {
                id: "QU".into(),
                symbol: "QU".into(),
                score: 10,
                is_blank: false,
                aliases: vec![],
            },
            eng::TileKind {
                id: "A".into(),
                symbol: "A".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert("QU".to_string(), 10);
    counts.insert("A".to_string(), 10);
    let cfg = eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: 7,
            height: 7,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 1,
        tile_counts: counts,
    };
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    st.dictionary = Some(Box::new(eng::FstDictionary::from_words(
        vec!["QUA".to_string()],
        true,
    )));
    assert!(st.dictionary.as_ref().unwrap().contains("QUA"));
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };
    // Empty board; rack has QU and A
    let rack = vec!["QU".to_string(), "A".to_string()];
    // Directly validate+score placing QU+A at center row
    let center = eng::CrosswordRules::center_cell(&st.board.geom);
    let right = st
        .board
        .geom
        .to_cell_id(eng::Coord2D {
            x: st.board.geom.from_cell_id(center).unwrap().x + 1,
            y: st.board.geom.from_cell_id(center).unwrap().y,
        })
        .unwrap();
    let mv = eng::MoveDraft {
        placements: vec![
            (
                center,
                eng::Tile {
                    kind_id: "QU".into(),
                    mark: None,
                },
            ),
            (
                right,
                eng::Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            ),
        ],
    };
    let v = rules.validate(&st, &mv).unwrap();
    let sc = rules.score(&st, &v);
    // direct scoring sanity
    assert!(sc.total >= 0);
    let moves = eng::generate_moves(&st, &rules, &rack, 7);
    assert!(
        moves
            .iter()
            .any(|cm| cm.word == "QUA" && cm.placements.len() == 2)
    );
}

fn cfg_with_dimensions(width: u32, height: u32) -> eng::GameConfig {
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
            eng::TileKind {
                id: "D".into(),
                symbol: "D".into(),
                score: 2,
                is_blank: false,
                aliases: vec![],
            },
            eng::TileKind {
                id: "L".into(),
                symbol: "L".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            },
            eng::TileKind {
                id: "E".into(),
                symbol: "E".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert("A".to_string(), 50);
    counts.insert("B".to_string(), 50);
    counts.insert("D".to_string(), 50);
    counts.insert("L".to_string(), 50);
    counts.insert("E".to_string(), 50);
    eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout { width, height },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 1,
        tile_counts: counts,
    }
}

fn cfg_basic() -> eng::GameConfig {
    cfg_with_dimensions(5, 5)
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
fn generate_3d_vertical_move() {
    let width = 3;
    let layer_h = 3;
    let depth = 2;
    let cfg = cfg_with_dimensions(width, layer_h * depth);
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    apply_3d_overlay(&mut st, width as i32, layer_h as i32, depth as i32);
    // Seed dictionary with AB so placement along Z axis is legal
    st.dictionary = Some(Box::new(eng::FstDictionary::from_words(
        vec!["AB".to_string()],
        true,
    )));
    let free_rules = eng::CrosswordRules {
        free_word_mode: true,
        require_center_first_move: false,
        ..Default::default()
    };
    let play_rules = eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let lower_coord = eng::Coord2D { x: 1, y: 1 };
    let lower_cell = st.board.geom.to_cell_id(lower_coord).unwrap();
    // place A at lower layer (z = 0)
    let mv = eng::MoveDraft {
        placements: vec![(
            lower_cell,
            eng::Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let v = free_rules.validate(&st, &mv).unwrap();
    let sc = free_rules.score(&st, &v);
    free_rules.commit(&mut st, v, &sc).unwrap();
    assert!(!st.board.cells[lower_cell.0 as usize].stack.is_empty());
    // Generate moves with rack containing B; expect placement directly above forming "AB"
    let rack = vec!["B".to_string()];
    let upper_cell = st
        .board
        .geom
        .to_cell_id(eng::Coord2D {
            x: lower_coord.x,
            y: lower_coord.y + layer_h as i32,
        })
        .unwrap();
    let moves = eng::generate_moves(&st, &play_rules, &rack, 7);
    assert!(moves.iter().any(|cm| {
        cm.placements.len() == 1 && cm.placements[0].0 == upper_cell && cm.score > 0
    }));
}

#[test]
fn generate_respects_stacking_cross_checks() {
    let mut st = eng::GameState::new(&cfg_basic(), 2).unwrap();
    let stacking_rules = eng::CrosswordRules {
        free_word_mode: true,
        stacking_enabled: true,
        require_center_first_move: false,
        ..Default::default()
    };
    let play_rules = eng::CrosswordRules {
        free_word_mode: false,
        stacking_enabled: true,
        ..Default::default()
    };
    let center = eng::CrosswordRules::center_cell(&st.board.geom);
    let center_coord = st.board.geom.from_cell_id(center).unwrap();
    let upper = st
        .board
        .geom
        .to_cell_id(eng::Coord2D {
            x: center_coord.x,
            y: center_coord.y - 1,
        })
        .unwrap();
    let lower = st
        .board
        .geom
        .to_cell_id(eng::Coord2D {
            x: center_coord.x,
            y: center_coord.y + 1,
        })
        .unwrap();
    // Place L above, A at center, then overlay E to create a stack (top E)
    let seed_moves = [(upper, "L"), (center, "A"), (center, "E")];
    for (cell, kid) in seed_moves {
        let mv = eng::MoveDraft {
            placements: vec![(
                cell,
                eng::Tile {
                    kind_id: kid.into(),
                    mark: None,
                },
            )],
        };
        let v = stacking_rules.validate(&st, &mv).unwrap();
        let sc = stacking_rules.score(&st, &v);
        stacking_rules.commit(&mut st, v, &sc).unwrap();
    }
    // dictionary only allows LED and L
    st.dictionary = Some(Box::new(eng::FstDictionary::from_words(
        vec!["LED".to_string(), "L".to_string()],
        true,
    )));
    let rack = vec!["D".to_string()];
    let moves = eng::generate_moves(&st, &play_rules, &rack, 7);
    assert!(
        moves.iter().any(|cm| {
            cm.word == "LED" && cm.placements.len() == 1 && cm.placements[0].0 == lower
        })
    );
}

// TODO: multi-character tile generation across orientations.
