use engine::{self as eng, AiConfig, Rules};
use std::time::Duration;

fn basic_config() -> eng::GameConfig {
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
                id: "T".into(),
                symbol: "T".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            },
            eng::TileKind {
                id: "O".into(),
                symbol: "O".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            },
            eng::TileKind {
                id: "Q".into(),
                symbol: "Q".into(),
                score: 10,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert("A".to_string(), 5);
    counts.insert("T".to_string(), 5);
    counts.insert("O".to_string(), 5);
    counts.insert("Q".to_string(), 1);
    eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: 5,
            height: 5,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "en".into(),
        rng_seed: 7,
        tile_counts: counts,
    }
}

fn attach_dictionary(st: &mut eng::GameState, words: &[&str]) {
    let dict = eng::FstDictionary::from_words(
        words.iter().map(|w| w.to_string()).collect::<Vec<_>>(),
        true,
    );
    st.dictionary = Some(Box::new(dict));
}

fn set_rack(st: &mut eng::GameState, tiles: &[&str]) {
    let pid = st.to_move.0;
    st.players[pid].rack.tiles.clear();
    for tile in tiles {
        st.players[pid].rack.tiles.push(eng::Tile {
            kind_id: tile.to_string(),
            mark: None,
        });
    }
}

#[test]
fn ai_prefers_better_leave() {
    let mut st = eng::GameState::new(&basic_config(), 1).unwrap();
    attach_dictionary(&mut st, &["AT", "TO"]);
    set_rack(&mut st, &["A", "T", "O"]);
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };

    let mut config = AiConfig::default();
    config.rack_leave.insert("O".into(), 5);
    config.rack_leave.insert("A".into(), 0);
    config.max_move_len = 7;

    let eval = eng::best_move_greedy(&st, &rules, &config).expect("should find a move");
    assert_eq!(eval.candidate.word, "AT");
    assert!(eval.total > eval.candidate.score);
}

#[test]
fn ai_deterministic_with_seed() {
    let mut st = eng::GameState::new(&basic_config(), 1).unwrap();
    attach_dictionary(&mut st, &["AT", "TA"]);
    set_rack(&mut st, &["A", "T"]);
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };

    let mut config = AiConfig::default();
    config.rack_leave.clear(); // tie on leave
    config.max_move_len = 7;
    config.randomness = Some(42);

    let first = eng::best_move_greedy(&st, &rules, &config).unwrap();
    let second = eng::best_move_greedy(&st, &rules, &config).unwrap();
    assert_eq!(first.candidate.word, second.candidate.word);
}

#[test]
fn ai_lookahead_considers_opponent_reply() {
    let mut st = eng::GameState::new(&basic_config(), 2).unwrap();
    attach_dictionary(&mut st, &["TA", "TO", "AT"]);
    // Seed board with a center A to create anchors
    let center = eng::CrosswordRules::center_cell(&st.board.geom);
    let free_rules = eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };
    let mv = eng::MoveDraft {
        placements: vec![(
            center,
            eng::Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let v = free_rules.validate(&st, &mv).unwrap();
    let sc = free_rules.score(&st, &v);
    free_rules.commit(&mut st, v, &sc).unwrap();
    st.to_move = eng::PlayerId(0);
    set_rack(&mut st, &["T"]);
    st.players[1].rack.tiles.clear();
    st.players[1].rack.tiles.push(eng::Tile {
        kind_id: "O".into(),
        mark: None,
    });
    let mut config = AiConfig::default();
    config.rack_leave.clear();
    config.max_move_len = 7;
    config.lookahead_depth = 1;
    let result = eng::best_move_greedy(&st, &free_rules, &config).unwrap();
    assert!(result.total < result.candidate.score + result.rack_leave + result.board_equity);
}

#[test]
fn ai_penalizes_endgame_leave() {
    let mut st = eng::GameState::new(&basic_config(), 1).unwrap();
    // Empty the bag to simulate the endgame scenario
    for count in st.bag.counts.values_mut() {
        *count = 0;
    }
    attach_dictionary(&mut st, &["A"]);
    set_rack(&mut st, &["A", "Q"]);
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };
    let cfg = AiConfig::default();
    let eval = eng::best_move_greedy(&st, &rules, &cfg).expect("expected a move");
    assert_eq!(eval.endgame_penalty, -10);
    assert_eq!(
        eval.total,
        eval.candidate.score + eval.rack_leave + eval.board_equity + eval.endgame_penalty
    );
    assert!(eval.total < eval.candidate.score);
}

#[test]
fn ai_hint_is_legal_under_strict_budget() {
    let mut st = eng::GameState::new(&basic_config(), 1).unwrap();
    attach_dictionary(&mut st, &["AT"]);
    set_rack(&mut st, &["A", "T"]);
    let rules = eng::CrosswordRules {
        free_word_mode: false,
        ..Default::default()
    };
    let mut cfg = AiConfig::default();
    cfg.max_nodes = Some(1);
    cfg.max_duration = Some(Duration::from_millis(0));
    let eval = eng::best_move_greedy(&st, &rules, &cfg).expect("hint available");
    let draft = eng::MoveDraft {
        placements: eval.candidate.placements.clone(),
    };
    let validated = rules.validate(&st, &draft).expect("move should validate");
    let score = rules.score(&st, &validated);
    assert_eq!(score.total, eval.candidate.score);
    let mut clone = st.clone();
    rules
        .commit(&mut clone, validated, &score)
        .expect("commit should succeed");
}
