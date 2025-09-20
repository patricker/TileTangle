use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use eng::{Dictionary, Rules};
use engine as eng;
use std::collections::HashMap;

fn sample_config() -> eng::GameConfig {
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
                id: "E".into(),
                symbol: "E".into(),
                score: 1,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let mut counts = HashMap::new();
    counts.insert("A".to_string(), 30);
    counts.insert("B".to_string(), 10);
    counts.insert("E".to_string(), 20);
    eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: 15,
            height: 15,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "bench".into(),
        rng_seed: 7,
        tile_counts: counts,
    }
}

fn benchmark_dictionary(c: &mut Criterion) {
    let words = ["AB", "ABE", "BA", "BEE", "BE", "CAB", "ACE"];
    let dict = eng::FstDictionary::from_words(words.iter().map(|w| w.to_string()), true);
    c.bench_function("dictionary_contains", |b| {
        b.iter(|| {
            black_box(dict.contains("ABE"));
            black_box(dict.contains("XYZ"));
        })
    });

    c.bench_function("dictionary_prefix", |b| {
        b.iter(|| {
            black_box(dict.has_prefix("AB"));
            black_box(dict.has_prefix("ZX"));
        })
    });
}

fn prepare_state(players: usize) -> eng::GameState {
    eng::GameState::new(&sample_config(), players).unwrap()
}

fn benchmark_move_generation(c: &mut Criterion) {
    c.bench_function("generate_moves_initial", |b| {
        b.iter_batched(
            || {
                let state = prepare_state(2);
                let rules = eng::CrosswordRules {
                    free_word_mode: true,
                    ..Default::default()
                };
                (state, rules)
            },
            |(state, rules)| {
                let pid = state.to_move.0;
                let rack: Vec<String> = state.players[pid]
                    .rack
                    .tiles
                    .iter()
                    .map(|t| t.kind_id.clone())
                    .collect();
                black_box(eng::generate_moves(&state, &rules, &rack, 7));
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("generate_moves_midgame", |b| {
        b.iter_batched(
            || {
                let mut state = prepare_state(2);
                let rules = eng::CrosswordRules {
                    free_word_mode: true,
                    ..Default::default()
                };
                // Seed a simple word to create anchors
                let center = eng::CrosswordRules::center_cell(&state.board.geom);
                let right = eng::CellId(center.0 + 1);
                let draft = eng::MoveDraft {
                    placements: vec![
                        (
                            center,
                            eng::Tile {
                                kind_id: "A".into(),
                                mark: None,
                            },
                        ),
                        (
                            right,
                            eng::Tile {
                                kind_id: "B".into(),
                                mark: None,
                            },
                        ),
                    ],
                };
                let validated = rules.validate(&state, &draft).unwrap();
                let score = rules.score(&state, &validated);
                rules.commit(&mut state, validated, &score).unwrap();
                (state, rules)
            },
            |(state, rules)| {
                let pid = state.to_move.0;
                let rack: Vec<String> = state.players[pid]
                    .rack
                    .tiles
                    .iter()
                    .map(|t| t.kind_id.clone())
                    .collect();
                black_box(eng::generate_moves(&state, &rules, &rack, 7));
            },
            BatchSize::SmallInput,
        );
    });
}

fn benchmark_ai(c: &mut Criterion) {
    c.bench_function("ai_greedy_depth0", |b| {
        b.iter_batched(
            || {
                let state = prepare_state(2);
                let rules = eng::CrosswordRules {
                    free_word_mode: true,
                    ..Default::default()
                };
                let cfg = eng::AiConfig {
                    parallel_eval: cfg!(feature = "parallel"),
                    ..Default::default()
                };
                (state, rules, cfg)
            },
            |(state, rules, cfg)| {
                black_box(eng::best_move_greedy(&state, &rules, &cfg));
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("ai_greedy_depth1", |b| {
        b.iter_batched(
            || {
                let state = prepare_state(2);
                let rules = eng::CrosswordRules {
                    free_word_mode: true,
                    ..Default::default()
                };
                let cfg = eng::AiConfig {
                    lookahead_depth: 1,
                    ..Default::default()
                };
                (state, rules, cfg)
            },
            |(state, rules, cfg)| {
                black_box(eng::best_move_greedy(&state, &rules, &cfg));
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    core_benches,
    benchmark_dictionary,
    benchmark_move_generation,
    benchmark_ai
);
criterion_main!(core_benches);
