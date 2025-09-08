use tiletangle_wasm as wasm_api;
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn cfg_json(width: u32, height: u32) -> String {
    serde_json::json!({
        "tileset": { "tile_kinds": [{"id":"A","symbol":"A","score":1}] },
        "rack_size": 7,
        "board_layout": {"width": width, "height": height},
        "ruleset_id": "cross",
        "dictionary_id": "en",
        "rng_seed": 1,
        "tile_counts": {"A": 0},
        "free_word_mode": true
    })
    .to_string()
}

#[wasm_bindgen_test]
pub fn empty_board_snapshot() {
    let game = wasm_api::new_game(&cfg_json(3, 2), 2).expect("new_game");
    let board = wasm_api::get_board(&game);
    let v: serde_json::Value = serde_json::from_str(&board).unwrap();
    let expected = serde_json::json!({
        "width": 3,
        "height": 2,
        "rows": [["", "", ""], ["", "", ""]],
    });
    assert_eq!(v, expected);
}

#[wasm_bindgen_test]
pub fn place_center_snapshot() {
    let mut game = wasm_api::new_game(&cfg_json(3, 3), 2).expect("new_game");
    let placements = serde_json::json!([{"x":1,"y":1,"kind_id":"A"}]).to_string();
    let _ = wasm_api::play_move(&mut game, &placements);
    let board = wasm_api::get_board(&game);
    let v: serde_json::Value = serde_json::from_str(&board).unwrap();
    let expected = serde_json::json!({
        "width": 3,
        "height": 3,
        "rows": [["", "", ""], ["", "A", ""], ["", "", ""]],
    });
    assert_eq!(v, expected);
}
