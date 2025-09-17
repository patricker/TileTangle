use engine::{self as eng, BoardGeometry, Rules};

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn board_after_move_matches_fixture() {
    let cfg_path = "../test_data/configs/config_small.json";
    let mv_path = "../test_data/moves/move_ab.json";
    let expect_path = "../test_data/expect/board_after_ab.json";

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
    let rules = eng::CrosswordRules {
        free_word_mode: cfg.free_word_mode,
        ..Default::default()
    };

    #[derive(serde::Deserialize)]
    struct JsPlacement {
        x: i32,
        y: i32,
        kind_id: String,
    }
    let placements: Vec<JsPlacement> = serde_json::from_str(&read(mv_path)).unwrap();
    let mut draft = eng::MoveDraft { placements: vec![] };
    for p in placements {
        let cid = state
            .board
            .geom
            .to_cell_id(eng::Coord2D { x: p.x, y: p.y })
            .unwrap();
        draft.placements.push((
            cid,
            eng::Tile {
                kind_id: p.kind_id,
                mark: None,
            },
        ));
    }
    let v = rules.validate(&state, &draft).unwrap();
    let sc = rules.score(&state, &v);
    assert!(sc.total >= 0);
    rules.commit(&mut state, v, &sc).unwrap();

    let w = state.board.geom.width as i32;
    let h = state.board.geom.height as i32;
    let mut rows: Vec<Vec<String>> = Vec::new();
    for y in 0..h {
        let mut row = Vec::new();
        for x in 0..w {
            let id = state.board.geom.to_cell_id(eng::Coord2D { x, y }).unwrap();
            let cell = &state.board.cells[id.0 as usize];
            let s = if let Some(t) = cell.stack.last() {
                t.kind_id.clone()
            } else {
                String::from("")
            };
            row.push(s);
        }
        rows.push(row);
    }
    let got = serde_json::json!({"width": w, "height": h, "rows": rows});
    let expect: serde_json::Value = serde_json::from_str(&read(expect_path)).unwrap();
    assert_eq!(got, expect);
}
