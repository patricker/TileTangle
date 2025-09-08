use engine::*;

fn main() {
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
        rng_seed: 42,
        tile_counts: counts,
    };
    let mut st = GameState::new(&cfg, 2).unwrap();
    let rules = CrosswordRules::default();
    let center = CrosswordRules::center_cell(&st.board.geom);
    st.board.bonuses.insert(
        center,
        Bonus {
            letter_mul: 1,
            word_mul: 2,
            tags: std::collections::BTreeSet::new(),
        },
    );
    let mv1 = MoveDraft {
        placements: vec![(
            center,
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let v1 = rules.validate(&st, &mv1).unwrap();
    let sc1 = rules.score(&st, &v1);
    println!(
        "Move 1: main='{}' score={} total={} bingo={}",
        sc1.main_word, sc1.main_score, sc1.total, sc1.bingo
    );
    rules.commit(&mut st, v1, &sc1).unwrap();

    let right = st
        .board
        .geom
        .to_cell_id(Coord2D {
            x: st.board.geom.from_cell_id(center).unwrap().x + 1,
            y: st.board.geom.from_cell_id(center).unwrap().y,
        })
        .unwrap();
    st.board.bonuses.insert(
        right,
        Bonus {
            letter_mul: 3,
            word_mul: 1,
            tags: std::collections::BTreeSet::new(),
        },
    );
    let mv2 = MoveDraft {
        placements: vec![(
            right,
            Tile {
                kind_id: "B".into(),
                mark: None,
            },
        )],
    };
    let v2 = rules.validate(&st, &mv2).unwrap();
    let sc2 = rules.score(&st, &v2);
    println!(
        "Move 2: main='{}' score={} total={} crosses={:?}",
        sc2.main_word, sc2.main_score, sc2.total, sc2.cross_words
    );
}
