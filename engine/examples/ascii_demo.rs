use engine::*;

fn main() {
    // Minimal 7x7 demo: draw rack and render empty board with a single preview placement
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
    counts.insert("A".to_string(), 30);
    counts.insert("B".to_string(), 2);
    let cfg = GameConfig {
        tileset,
        rack_size: 7,
        board_layout: RectBoardLayout {
            width: 7,
            height: 7,
        },
        ruleset_id: "crossword_classic".into(),
        dictionary_id: "en_demo".into(),
        rng_seed: 42,
        tile_counts: counts,
    };
    let state = GameState::new(&cfg, 2).expect("state");
    render_board(&state.board);

    let center = state.board.geom.to_cell_id(Coord2D { x: 3, y: 3 }).unwrap();
    let draft = MoveDraft {
        placements: vec![(
            center,
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
        )],
    };
    let nb = state.preview(&draft).unwrap();
    println!("\nAfter preview:\n");
    render_board(&nb);
}

fn render_board(board: &Board<RectGridGeometry>) {
    let w = board.geom.width as i32;
    let h = board.geom.height as i32;
    for y in 0..h {
        for x in 0..w {
            let id = board.geom.to_cell_id(Coord2D { x, y }).unwrap();
            let cell = &board.cells[id.0 as usize];
            let ch = if let Some(t) = cell.stack.last() {
                &t.kind_id
            } else {
                "."
            };
            print!("{} ", ch.chars().next().unwrap_or('.'));
        }
        println!();
    }
}
