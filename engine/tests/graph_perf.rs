use engine as eng;
use engine::BoardGeometry;
use engine::Rules;
use std::time::{Duration, Instant};

fn make_cfg(w: u32, h: u32) -> eng::GameConfig {
    let tileset = eng::Tileset {
        tile_kinds: vec![eng::TileKind {
            id: "A".into(),
            symbol: "A".into(),
            score: 1,
            is_blank: false,
            aliases: vec![],
        }],
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert("A".into(), 10_000);
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

/// Ensure graph-based validation scales linearly on a large sparse line graph.
#[test]
fn graph_validation_large_sparse_linear_within_bounds() {
    // Build a single long line along y=0 with E-tagged edges.
    let n: usize = 2000; // number of nodes in line
    let cfg = make_cfg((n as u32) + 1, 1);
    let mut st = eng::GameState::new(&cfg, 2).unwrap();
    let nodes: Vec<eng::Coord2D> = (0..n as i32).map(|x| eng::Coord2D { x, y: 0 }).collect();
    let mut edges: Vec<(usize, usize, String)> = Vec::with_capacity(n.saturating_sub(1));
    for i in 0..(n - 1) {
        edges.push((i, i + 1, "E".to_string()));
    }
    st.apply_graph_overlay(eng::GraphOverlay { nodes, edges })
        .unwrap();

    // Draft: place a contiguous sequence of tiles along the entire line.
    let mut placements: Vec<(eng::CellId, eng::Tile)> = Vec::with_capacity(n);
    for x in 0..n as i32 {
        let id = st.board.geom.to_cell_id(eng::Coord2D { x, y: 0 }).unwrap();
        placements.push((
            id,
            eng::Tile {
                kind_id: "A".into(),
                mark: None,
            },
        ));
    }
    let mv = eng::MoveDraft { placements };
    let rules = eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };

    // Validate and score within a reasonable bound on CI hardware.
    let start = Instant::now();
    let v = rules.validate(&st, &mv).unwrap();
    let sc = rules.score(&st, &v);
    assert!(sc.total >= 0);
    let elapsed = start.elapsed();

    // Bound: this should comfortably be well under 500ms on typical CI runners.
    // Keep the bound loose to avoid flakiness.
    assert!(
        elapsed < Duration::from_millis(500),
        "graph validation too slow: {:?}",
        elapsed
    );
}
