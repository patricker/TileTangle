use tiletangle_wasm as wasm_api;
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
pub fn new_game_and_get_board() {
    let cfg = serde_json::json!({
        "tileset": { "tile_kinds": [{"id":"A","symbol":"A","score":1}] },
        "rack_size": 7,
        "board_layout": {"width": 3, "height": 3},
        "ruleset_id": "cross",
        "dictionary_id": "en",
        "rng_seed": 1,
        "tile_counts": {"A": 5},
        "free_word_mode": true
    });
    let game = wasm_api::new_game(&cfg.to_string(), 2).unwrap();
    let b = wasm_api::get_board(&game);
    let v: serde_json::Value = serde_json::from_str(&b).unwrap();
    assert_eq!(v["width"], 3);
    assert_eq!(v["height"], 3);
}
