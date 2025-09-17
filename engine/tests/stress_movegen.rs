use engine as eng;
use engine::BoardGeometry;

#[test]
fn stress_many_anchors_long_rack_small_board() {
    // 7x7 board, place a plus-shape of A's to create many anchors
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
    counts.insert("A".to_string(), 100);
    counts.insert("B".to_string(), 100);
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
    // Dict allows a few sequences to avoid explosion
    let dict = eng::FstDictionary::from_words(
        vec![
            "AA".to_string(),
            "ABA".to_string(),
            "BAB".to_string(),
            "AB".to_string(),
            "BA".to_string(),
        ],
        true,
    );
    st.dictionary = Some(Box::new(dict));
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };
    // Preplace plus shape at center
    let c = eng::CrosswordRules::center_cell(&st.board.geom);
    let cc = st.board.geom.from_cell_id(c).unwrap();
    let mut pre = vec![c];
    pre.push(
        st.board
            .geom
            .to_cell_id(eng::Coord2D {
                x: cc.x - 1,
                y: cc.y,
            })
            .unwrap(),
    );
    pre.push(
        st.board
            .geom
            .to_cell_id(eng::Coord2D {
                x: cc.x + 1,
                y: cc.y,
            })
            .unwrap(),
    );
    pre.push(
        st.board
            .geom
            .to_cell_id(eng::Coord2D {
                x: cc.x,
                y: cc.y - 1,
            })
            .unwrap(),
    );
    pre.push(
        st.board
            .geom
            .to_cell_id(eng::Coord2D {
                x: cc.x,
                y: cc.y + 1,
            })
            .unwrap(),
    );
    for id in pre {
        st.board.cells[id.0 as usize].stack.push(eng::Tile {
            kind_id: "A".into(),
            mark: None,
        });
    }
    // Long rack
    let rack = vec![
        "A".to_string(),
        "B".to_string(),
        "A".to_string(),
        "B".to_string(),
        "A".to_string(),
        "B".to_string(),
        "A".to_string(),
    ];
    let moves = eng::generate_moves(&st, &rules, &rack, 7);
    // Just a smoke assertion: non-empty and not absurdly large on this small dict
    assert!(!moves.is_empty());
    assert!(moves.len() < 500);
}
