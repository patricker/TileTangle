//! Example external plugin crate for TileTangle.
//!
//! Provides two simple `RulePlugin` implementations:
//! - `ScoreBonusPlugin` adds a fixed bonus to every scored move.
//! - `RejectFarPlacementsPlugin` rejects moves where any two placed tiles are more than N cells apart (Manhattan).

use engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBonusPlugin {
    pub bonus: i32,
}

impl Default for ScoreBonusPlugin {
    fn default() -> Self {
        Self { bonus: 5 }
    }
}

impl engine::RulePlugin for ScoreBonusPlugin {
    fn name(&self) -> &str {
        "score_bonus"
    }
    fn modify_score(
        &self,
        _state: &engine::GameState,
        _v: &engine::ValidatedMove,
        sc: &mut engine::ScoreBreakdown,
    ) {
        sc.total += self.bonus;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectFarPlacementsPlugin {
    pub max_manhattan: i32,
}

impl Default for RejectFarPlacementsPlugin {
    fn default() -> Self {
        Self { max_manhattan: 7 }
    }
}

impl engine::RulePlugin for RejectFarPlacementsPlugin {
    fn name(&self) -> &str {
        "reject_far"
    }
    fn validate(
        &self,
        state: &engine::GameState,
        mv: &engine::UserMove,
    ) -> Result<(), engine::EngineError> {
        // Gather coordinates from actions (only Place/Stack considered)
        let mut coords: Vec<engine::Coord2D> = Vec::new();
        for a in &mv.actions {
            match a {
                engine::Action::Place { x, y, .. } | engine::Action::Stack { x, y, .. } => {
                    coords.push(engine::Coord2D { x: *x, y: *y })
                }
                _ => {}
            }
        }
        for i in 0..coords.len() {
            for j in i + 1..coords.len() {
                let d = (coords[i].x - coords[j].x).abs() + (coords[i].y - coords[j].y).abs();
                if d > self.max_manhattan {
                    return Err(engine::EngineError::Config("placements too far apart"));
                }
            }
        }
        Ok(())
    }
}
