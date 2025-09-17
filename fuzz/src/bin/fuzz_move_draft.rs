#![no_main]

use libfuzzer_sys::fuzz_target;
use tiletangle_engine as eng;

fn seed_config() -> eng::GameConfig {
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
        ],
    };
    let mut counts = std::collections::HashMap::new();
    counts.insert("A".to_string(), 20);
    counts.insert("B".to_string(), 20);
    eng::GameConfig {
        tileset,
        rack_size: 7,
        board_layout: eng::RectBoardLayout { width: 15, height: 15 },
        ruleset_id: "cross".into(),
        dictionary_id: "fuzz".into(),
        rng_seed: 99,
        tile_counts: counts,
    }
}

fuzz_target!(|data: &[u8]| {
    let mut state = match eng::GameState::new(&seed_config(), 2) {
        Ok(s) => s,
        Err(_) => return,
    };
    let rules = eng::CrosswordRules { free_word_mode: true, ..Default::default() };
    if data.is_empty() {
        return;
    }
    let center = eng::CrosswordRules::center_cell(&state.board.geom);
    let width = state.board.geom.width as i32;
    // derive placements along a simple line anchored at center
    let mut draft = eng::MoveDraft { placements: Vec::new() };
    let mut offset = 0i32;
    for chunk in data.chunks(2).take(7) {
        let letter = (chunk[0] & 0x1F) % 2; // 0 -> A, 1 -> B
        let kind = if letter == 0 { "A" } else { "B" };
        let dx = if chunk.len() > 1 { (chunk[1] as i32 % 3) - 1 } else { 0 };
        let x = center.0 as i32 + offset;
        let y = center.0 as i32 + dx;
        if x < 0 || x >= width || y < 0 || y >= width {
            continue;
        }
        if let Some(cell) = state.board.geom.to_cell_id(eng::Coord2D { x, y }) {
            draft.placements.push((cell, eng::Tile { kind_id: kind.into(), mark: None }));
        }
        offset += 1;
    }
    if draft.placements.is_empty() {
        return;
    }
    // attempt to validate and score; ignore errors
    if let Ok(validated) = rules.validate(&state, &draft) {
        let score = rules.score(&state, &validated);
        let _ = rules.commit(&mut state, validated, &score);
    }
});
