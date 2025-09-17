use engine::*;

fn main() {
    // Dictionary only allows AB
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
        ],
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert("A".to_string(), 10);
    counts.insert("B".to_string(), 10);
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
        tile_counts: counts,
    };
    let state = GameState::new(&cfg, 2).unwrap();
    let mut state = state;
    let rules = CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };
    state.dictionary = Some(Box::new(SetDictionary::from_words(
        vec!["AB".to_string()],
        true,
    )));
    // Try to place A at center -> invalid dictionary
    let c = CrosswordRules::center_cell(&state.board.geom);
    let mv = MoveDraft {
        placements: vec![(
            c,
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let v = rules.validate(&state, &mv).unwrap();
    let sc = rules.score(&state, &v);
    if sc.main_score < 0 {
        eprintln!("Rejected: word not in dictionary");
    } else {
        println!("Accepted: total {}", sc.total);
    }
}
