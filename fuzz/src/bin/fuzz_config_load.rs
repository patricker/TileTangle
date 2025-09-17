#![no_main]

use libfuzzer_sys::fuzz_target;
use engine::{GameConfig, GameState};

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(cfg) = serde_json::from_str::<GameConfig>(text) {
            let _ = GameState::new(&cfg, 2);
        }
    }
});
