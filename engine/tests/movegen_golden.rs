use engine::{self as eng, BoardGeometry, Rules};

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn golden_moves_ab() {
    // Load small config
    let cfg_path = "../test_data/configs/config_small.json";
    let cfg_json = read(cfg_path);
    #[derive(serde::Deserialize)]
    struct JsTileKind {
        id: String,
        symbol: String,
        score: i16,
        #[serde(default)]
        is_blank: bool,
        #[serde(default)]
        aliases: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    struct JsTileset {
        tile_kinds: Vec<JsTileKind>,
    }
    #[derive(serde::Deserialize)]
    struct JsRect {
        width: u32,
        height: u32,
    }
    #[derive(serde::Deserialize)]
    struct JsCfg {
        tileset: JsTileset,
        rack_size: usize,
        board_layout: JsRect,
        ruleset_id: String,
        dictionary_id: String,
        rng_seed: u64,
        tile_counts: std::collections::HashMap<String, u32>,
        #[serde(default)]
        free_word_mode: bool,
    }
    let cfg: JsCfg = serde_json::from_str(&cfg_json).unwrap();
    let tileset = eng::Tileset {
        tile_kinds: cfg
            .tileset
            .tile_kinds
            .into_iter()
            .map(|k| eng::TileKind {
                id: k.id,
                symbol: k.symbol,
                score: k.score,
                is_blank: k.is_blank,
                aliases: k.aliases,
            })
            .collect(),
    };
    let ecfg = eng::GameConfig {
        tileset,
        rack_size: cfg.rack_size,
        board_layout: eng::RectBoardLayout {
            width: cfg.board_layout.width,
            height: cfg.board_layout.height,
        },
        ruleset_id: cfg.ruleset_id,
        dictionary_id: cfg.dictionary_id,
        rng_seed: cfg.rng_seed,
        tile_counts: cfg.tile_counts,
    };
    let mut state = eng::GameState::new(&ecfg, 2).unwrap();
    let mut rules = eng::CrosswordRules {
        free_word_mode: true,
        ..Default::default()
    };

    // Load test scenario
    #[derive(serde::Deserialize)]
    struct JsPlacement {
        x: i32,
        y: i32,
        kind_id: String,
    }
    #[derive(serde::Deserialize)]
    struct JsMovesFixture {
        preplacements: Vec<JsPlacement>,
        rack: Vec<String>,
        dict_words: Vec<String>,
        expected_words: Vec<String>,
    }
    let fx: JsMovesFixture =
        serde_json::from_str(&read("../test_data/moves/golden_moves_ab.json")).unwrap();
    // Preplace
    for p in fx.preplacements {
        let cid = state
            .board
            .geom
            .to_cell_id(eng::Coord2D { x: p.x, y: p.y })
            .unwrap();
        let mv = eng::MoveDraft {
            placements: vec![(
                cid,
                eng::Tile {
                    kind_id: p.kind_id,
                    mark: None,
                },
            )],
        };
        let v = rules.validate(&state, &mv).unwrap();
        let sc = rules.score(&state, &v);
        rules.commit(&mut state, v, &sc).unwrap();
    }
    // Dictionary
    state.dictionary = Some(Box::new(eng::FstDictionary::from_words(
        fx.dict_words.clone(),
        true,
    )));
    // Generate moves (dictionary now enforced by rules)
    rules.free_word_mode = false;
    let moves = eng::generate_moves(&state, &rules, &fx.rack, 7);
    let mut words: Vec<String> = moves.into_iter().map(|m| m.word).collect();
    words.sort();
    words.dedup();
    let mut expected = fx.expected_words.clone();
    expected.sort();
    expected.dedup();
    assert_eq!(words, expected);
}
