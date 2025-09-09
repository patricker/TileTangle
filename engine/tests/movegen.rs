use engine::{self as eng, BoardGeometry, Rules};

#[test]
fn generate_simple_horizontal_move() {
    // Tiles: A(1), B(3); dict allows AB only
    let tileset = eng::Tileset { tile_kinds: vec![
        eng::TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
        eng::TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
    ] };
    let mut counts = std::collections::HashMap::new();
    counts.insert("A".to_string(), 10);
    counts.insert("B".to_string(), 10);
    let cfg = eng::GameConfig { tileset, rack_size: 7, board_layout: eng::RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 1, tile_counts: counts };
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    st.dictionary = Some(Box::new(eng::FstDictionary::from_words(vec!["AB".to_string()], true)));
    let rules = eng::CrosswordRules { free_word_mode: false, ..Default::default() };
    // Seed board with A at center (free commit)
    let center = eng::CrosswordRules::center_cell(&st.board.geom);
    let mv = eng::MoveDraft { placements: vec![(center, eng::Tile { kind_id: "A".into(), mark: None })] };
    let fv = eng::CrosswordRules { free_word_mode: true, ..Default::default() }.validate(&st, &mv).unwrap();
    let fs = eng::CrosswordRules { free_word_mode: true, ..Default::default() }.score(&st, &fv);
    eng::CrosswordRules { free_word_mode: true, ..Default::default() }.commit(&mut st, fv, &fs).unwrap();
    // Rack has B
    let rack = vec!["B".to_string()];
    let moves = eng::generate_moves(&st, &rules, &rack, 7);
    assert!(moves.iter().any(|cm| cm.word == "AB" && cm.placements.len() == 1));
}

#[test]
fn generate_uses_blank_for_letter() {
    // Tiles: Blank(0), B(3); dict allows AB only
    let tileset = eng::Tileset { tile_kinds: vec![
        eng::TileKind { id: "BL".into(), symbol: "_".into(), score: 0, is_blank: true, aliases: vec![] },
        eng::TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
    ] };
    let mut counts = std::collections::HashMap::new();
    counts.insert("BL".to_string(), 10);
    counts.insert("B".to_string(), 10);
    let cfg = eng::GameConfig { tileset, rack_size: 7, board_layout: eng::RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 1, tile_counts: counts };
    let st0 = eng::GameState::new(&cfg, 2).unwrap();
    let mut st = st0;
    st.dictionary = Some(Box::new(eng::FstDictionary::from_words(vec!["AB".to_string()], true)));
    let rules = eng::CrosswordRules { free_word_mode: false, ..Default::default() };
    // Empty board; rack has Blank and B
    let rack = vec!["BL".to_string(), "B".to_string()];
    let moves = eng::generate_moves(&st, &rules, &rack, 7);
    assert!(moves.iter().any(|cm| cm.word == "AB" && cm.placements.len() == 2));
}

// TODO: multi-character tile generation across orientations.
