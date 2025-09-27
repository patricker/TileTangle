use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use eng::BoardGeometry;
use engine as eng;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn load_words(limit: usize) -> Vec<String> {
    // Resolve repo-rooted path: engine benches run with CWD at the crate dir
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // repo root
    path.push("assets/dictionaries/TWL06.txt");
    let f = File::open(&path).expect("open TWL06.txt");
    let reader = BufReader::new(f);
    let mut out = Vec::with_capacity(limit);
    for line in reader.lines() {
        if out.len() >= limit {
            break;
        }
        let s = line.expect("read line");
        let w = s.trim();
        if w.is_empty() || w.starts_with('#') {
            continue;
        }
        out.push(w.to_string());
    }
    out
}

fn hex_overlay_state() -> eng::GameState {
    // Minimal A/B/C tiles for synthetic workload
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
                id: "C".into(),
                symbol: "C".into(),
                score: 3,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let mut counts = HashMap::new();
    counts.insert("A".to_string(), 40);
    counts.insert("B".to_string(), 30);
    counts.insert("C".to_string(), 30);
    let cfg = eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout {
            width: 7,
            height: 7,
        },
        ruleset_id: "cross".into(),
        dictionary_id: "bench".into(),
        rng_seed: 11,
        tile_counts: counts,
    };
    let mut state = eng::GameState::new(&cfg, 2).unwrap();

    // Even-r hex overlay (E, W, NE, NW, SE, SW)
    let w = 7i32;
    let h = 7i32;
    let mut nodes = Vec::new();
    for y in 0..h {
        for x in 0..w {
            nodes.push(eng::Coord2D { x, y });
        }
    }
    let idx = |x: i32, y: i32| -> usize { (y * w + x) as usize };
    let mut edges: Vec<(usize, usize, String)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let even = y % 2 == 0;
            let east_shift = if even { 0 } else { 1 };
            let west_shift = if even { -1 } else { 0 };
            let mut add = |x1: i32, y1: i32, x2: i32, y2: i32, dir: &str| {
                if x2 < 0 || x2 >= w || y2 < 0 || y2 >= h {
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
        .apply_graph_overlay(eng::GraphOverlay { nodes, edges })
        .unwrap();

    // Seed center anchor with an 'A'
    let c = state
        .board
        .geom
        .to_cell_id(eng::Coord2D { x: 3, y: 3 })
        .unwrap();
    state.board.cells[c.0 as usize].stack.push(eng::Tile {
        kind_id: "A".into(),
        mark: None,
    });
    state
}

fn bench_movegen_hex_fst(c: &mut Criterion) {
    c.bench_function("movegen_hex_fst", |b| {
        b.iter_batched(
            || {
                let mut st = hex_overlay_state();
                // Larger lexicon slice to better reflect real pruning
                let words = load_words(50_000);
                let opts = eng::DictionaryOptions {
                    case_fold: true,
                    ..Default::default()
                };
                let dict = eng::FstDictionary::from_words_opts(words, opts);
                st.dictionary = Some(Box::new(dict) as Box<dyn eng::Dictionary + Send + Sync>);
                let rules = eng::CrosswordRules {
                    free_word_mode: false,
                    ..Default::default()
                };
                // Use player 0 rack as input to movegen
                let rack: Vec<String> = st.players[0]
                    .rack
                    .tiles
                    .iter()
                    .map(|t| t.kind_id.clone())
                    .collect();
                (st, rules, rack)
            },
            |(st, rules, rack)| {
                black_box(eng::generate_moves(&st, &rules, &rack, 7));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_movegen_hex_gaddag(c: &mut Criterion) {
    c.bench_function("movegen_hex_gaddag", |b| {
        b.iter_batched(
            || {
                let mut st = hex_overlay_state();
                let words = load_words(50_000);
                let opts = eng::DictionaryOptions {
                    case_fold: true,
                    ..Default::default()
                };
                let dict = eng::GaddagDictionary::from_words_opts(words, opts);
                st.dictionary = Some(Box::new(dict) as Box<dyn eng::Dictionary + Send + Sync>);
                let rules = eng::CrosswordRules {
                    free_word_mode: false,
                    ..Default::default()
                };
                let rack: Vec<String> = st.players[0]
                    .rack
                    .tiles
                    .iter()
                    .map(|t| t.kind_id.clone())
                    .collect();
                (st, rules, rack)
            },
            |(st, rules, rack)| {
                // Note: current graph movegen path does not use GADDAG arcs; this serves as a baseline.
                black_box(eng::generate_moves(&st, &rules, &rack, 7));
            },
            BatchSize::SmallInput,
        );
    });
}

fn rect_state_with_anchor(width: u32, height: u32) -> eng::GameState {
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
                id: "C".into(),
                symbol: "C".into(),
                score: 3,
                is_blank: false,
                aliases: vec![],
            },
        ],
    };
    let mut counts = HashMap::new();
    counts.insert("A".to_string(), 40);
    counts.insert("B".to_string(), 30);
    counts.insert("C".to_string(), 30);
    let cfg = eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout { width, height },
        ruleset_id: "cross".into(),
        dictionary_id: "bench".into(),
        rng_seed: 13,
        tile_counts: counts,
    };
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    let c = eng::CrosswordRules::center_cell(&st.board.geom);
    st.board.cells[c.0 as usize].stack.push(eng::Tile {
        kind_id: "A".into(),
        mark: None,
    });
    st
}

fn bench_movegen_rect_fst(c: &mut Criterion) {
    c.bench_function("movegen_rect_fst", |b| {
        b.iter_batched(
            || {
                let mut st = rect_state_with_anchor(15, 15);
                let words = load_words(50_000);
                let opts = eng::DictionaryOptions {
                    case_fold: true,
                    ..Default::default()
                };
                let dict = eng::FstDictionary::from_words_opts(words, opts);
                st.dictionary = Some(Box::new(dict) as Box<dyn eng::Dictionary + Send + Sync>);
                let rules = eng::CrosswordRules {
                    free_word_mode: false,
                    ..Default::default()
                };
                let rack: Vec<String> = st.players[0]
                    .rack
                    .tiles
                    .iter()
                    .map(|t| t.kind_id.clone())
                    .collect();
                (st, rules, rack)
            },
            |(st, rules, rack)| {
                black_box(eng::generate_moves(&st, &rules, &rack, 7));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_movegen_rect_gaddag(c: &mut Criterion) {
    c.bench_function("movegen_rect_gaddag", |b| {
        b.iter_batched(
            || {
                let mut st = rect_state_with_anchor(15, 15);
                let words = load_words(50_000);
                let opts = eng::DictionaryOptions {
                    case_fold: true,
                    ..Default::default()
                };
                let dict = eng::GaddagDictionary::from_words_opts(words, opts);
                st.dictionary = Some(Box::new(dict) as Box<dyn eng::Dictionary + Send + Sync>);
                let rules = eng::CrosswordRules {
                    free_word_mode: false,
                    ..Default::default()
                };
                let rack: Vec<String> = st.players[0]
                    .rack
                    .tiles
                    .iter()
                    .map(|t| t.kind_id.clone())
                    .collect();
                (st, rules, rack)
            },
            |(st, rules, rack)| {
                black_box(eng::generate_moves(&st, &rules, &rack, 7));
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    movegen_graph,
    bench_movegen_rect_fst,
    bench_movegen_rect_gaddag,
    bench_movegen_hex_fst,
    bench_movegen_hex_gaddag
);
criterion_main!(movegen_graph);
