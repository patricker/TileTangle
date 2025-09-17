use engine::{
    self as eng, CrosswordRules, GameConfig, GameState, MoveDraft, Rules, Tile, TileKind, Tileset,
};

fn sample_config() -> GameConfig {
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
    GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: 5,
            height: 5,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "test".into(),
        rng_seed: 42,
        tile_counts: counts,
    }
}

fn seeded_state(players: usize) -> GameState {
    GameState::new(&sample_config(), players).unwrap()
}

#[test]
fn snapshot_round_trip_json() {
    let mut state = seeded_state(2);
    // Seed racks for a simple move "AB" on center row
    let center = eng::CrosswordRules::center_cell(&state.board.geom);
    let right = eng::CellId(center.0 + 1);
    state.players[0].rack.tiles = vec![
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "B".into(),
            mark: None,
        },
    ];
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let draft = MoveDraft {
        placements: vec![
            (
                center,
                Tile {
                    kind_id: "A".into(),
                    mark: None,
                },
            ),
            (
                right,
                Tile {
                    kind_id: "B".into(),
                    mark: None,
                },
            ),
        ],
    };
    let validated = rules.validate(&state, &draft).unwrap();
    let score = rules.score(&state, &validated);
    rules.commit(&mut state, validated, &score).unwrap();

    let json = state.snapshot_json().unwrap();
    let restored = GameState::from_snapshot_json(&json).unwrap();

    assert_eq!(state.board.cells, restored.board.cells);
    assert_eq!(state.players[0].score, restored.players[0].score);
    assert_eq!(state.players[0].rack.tiles, restored.players[0].rack.tiles);
    assert_eq!(state.turn_num, restored.turn_num);
    assert_eq!(
        state.compute_position_hash(),
        restored.compute_position_hash()
    );
    assert_eq!(state.event_log.len(), restored.event_log.len());
}

#[test]
fn snapshot_round_trip_cbor() {
    let mut state = seeded_state(1);
    state.players[0].rack.tiles = vec![Tile {
        kind_id: "A".into(),
        mark: None,
    }];
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let draft = MoveDraft {
        placements: vec![(
            eng::CrosswordRules::center_cell(&state.board.geom),
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let validated = rules.validate(&state, &draft).unwrap();
    let score = rules.score(&state, &validated);
    rules.commit(&mut state, validated, &score).unwrap();

    let cbor = state.snapshot_cbor().unwrap();
    let restored = GameState::from_snapshot_cbor(&cbor).unwrap();
    assert_eq!(
        state.compute_position_hash(),
        restored.compute_position_hash()
    );
    assert_eq!(state.event_log.len(), restored.event_log.len());
    assert_eq!(state.players[0].score, restored.players[0].score);
}

#[test]
fn position_hash_changes_after_move() {
    let mut state = seeded_state(2);
    state.players[0].rack.tiles = vec![Tile {
        kind_id: "A".into(),
        mark: None,
    }];
    let initial = state.compute_position_hash();
    let center = eng::CrosswordRules::center_cell(&state.board.geom);
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let draft = MoveDraft {
        placements: vec![(
            center,
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let validated = rules.validate(&state, &draft).unwrap();
    let score = rules.score(&state, &validated);
    rules.commit(&mut state, validated, &score).unwrap();
    let after = state.compute_position_hash();
    assert_ne!(initial, after);
}

#[test]
fn event_log_includes_play_and_pass() {
    let mut state = seeded_state(2);
    state.players[0].rack.tiles = vec![Tile {
        kind_id: "A".into(),
        mark: None,
    }];
    let center = eng::CrosswordRules::center_cell(&state.board.geom);
    let rules = CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let draft = MoveDraft {
        placements: vec![(
            center,
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let validated = rules.validate(&state, &draft).unwrap();
    let score = rules.score(&state, &validated);
    rules.commit(&mut state, validated, &score).unwrap();

    state.pass_turn();
    assert!(
        state
            .event_log
            .iter()
            .any(|e| matches!(e.kind, eng::GameEventKind::Play { .. }))
    );
    assert!(
        state
            .event_log
            .iter()
            .any(|e| matches!(e.kind, eng::GameEventKind::Pass))
    );
}

#[test]
fn exchange_tiles_logs_event_and_updates_hash() {
    let mut state = seeded_state(1);
    state.players[0].rack.tiles = vec![
        Tile {
            kind_id: "A".into(),
            mark: None,
        },
        Tile {
            kind_id: "B".into(),
            mark: None,
        },
    ];
    let before = state.compute_position_hash();
    let drawn = state.exchange_tiles(&["A".into()]).unwrap();
    assert_eq!(drawn.len(), 1);
    let after = state.compute_position_hash();
    assert_ne!(before, after);
    assert!(
        state
            .event_log
            .iter()
            .any(|e| matches!(e.kind, eng::GameEventKind::Exchange { .. }))
    );
}
