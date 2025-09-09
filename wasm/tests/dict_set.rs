use wasm_bindgen_test::*;
use tiletangle_wasm::{new_game, set_dictionary_from_text, play_move};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn set_dict_and_reject_invalid() {
    use wasm_bindgen::JsValue;
    let cfg = serde_json::json!({
        "tileset": {"tile_kinds": [
            {"id": "A", "symbol": "A", "score": 1},
            {"id": "B", "symbol": "B", "score": 3}
        ]},
        "rack_size": 7,
        "board_layout": {"width": 5, "height": 5},
        "ruleset_id": "cross",
        "dictionary_id": "en",
        "rng_seed": 42,
        "tile_counts": {"A": 10, "B": 10},
        "free_word_mode": false
    });
    let mut game = new_game(&cfg.to_string(), 2).unwrap();
    // dict only allows AB
    set_dictionary_from_text(&mut game, "AB\n", true).unwrap();
    // single A at center should be rejected
    let placements = serde_json::json!([
        {"x": 2, "y": 2, "kind_id": "A"}
    ]).to_string();
    let err = play_move(&mut game, &placements).unwrap_err();
    let s = format!("{:?}", err);
    assert!(s.to_lowercase().contains("invalid"));
}
