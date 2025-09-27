use std::collections::HashMap;
use std::time::Duration;

use crate::{GameState, Rules};

// The AI module now follows a strategy pattern. The existing greedy implementation
// is preserved in `greedy.rs`, and public functions here dispatch to that default.

mod types;
mod heuristics;
mod greedy;
pub use types::EvaluatedMove;
pub use heuristics::evaluate_candidate_move;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub rack_leave: HashMap<String, i32>,
    pub randomness: Option<u64>,
    pub max_move_len: usize,
    pub lookahead_depth: usize,
    pub max_nodes: Option<usize>,
    pub max_duration: Option<Duration>,
    pub candidate_limit: Option<usize>,
    pub reply_move_limit: usize,
    pub noise_range: i32,
    pub parallel_eval: bool,
    pub opponent_model: OpponentModel,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            rack_leave: default_rack_leave_table(),
            randomness: None,
            max_move_len: 15,
            lookahead_depth: 0,
            max_nodes: None,
            max_duration: None,
            candidate_limit: None,
            reply_move_limit: usize::MAX,
            noise_range: 0,
            parallel_eval: false,
            opponent_model: OpponentModel::PerfectInfo,
        }
    }
}

impl AiConfig {
    fn requires_rng(&self) -> bool { self.noise_range > 0 }

    pub fn for_difficulty(level: AiDifficulty) -> Self {
        let mut cfg = Self::default();
        cfg.apply_difficulty(level);
        cfg
    }

    pub fn apply_difficulty(&mut self, level: AiDifficulty) {
        match level {
            AiDifficulty::Easy => {
                self.lookahead_depth = 0;
                self.max_nodes = Some(16);
                self.max_duration = Some(Duration::from_millis(5));
                self.candidate_limit = Some(20);
                self.reply_move_limit = 6;
                self.noise_range = 12;
                self.parallel_eval = false;
            }
            AiDifficulty::Medium => {
                self.lookahead_depth = 0;
                self.max_nodes = Some(64);
                self.max_duration = Some(Duration::from_millis(25));
                self.candidate_limit = Some(32);
                self.reply_move_limit = 12;
                self.noise_range = 4;
                self.parallel_eval = false;
            }
            AiDifficulty::Hard => {
                self.lookahead_depth = 1;
                self.max_nodes = None;
                self.max_duration = None;
                self.candidate_limit = None;
                self.reply_move_limit = usize::MAX;
                self.noise_range = 0;
                self.parallel_eval = false;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpponentModel {
    /// Use the actual opponent rack from the state (perfect information).
    PerfectInfo,
    /// Treat the opponent rack as hidden: return their tiles to the bag and draw a fresh rack from the bag
    /// for reply simulation. This is a simple bag-sampling model (single sample).
    BagSampling,
}

fn default_rack_leave_table() -> HashMap<String, i32> {
    HashMap::from([
        ("A".into(), 1),
        ("E".into(), 1),
        ("I".into(), 1),
        ("L".into(), 1),
        ("N".into(), 1),
        ("R".into(), 1),
        ("S".into(), 1),
        ("T".into(), 1),
        ("O".into(), 0),
        ("D".into(), -1),
        ("G".into(), -1),
        ("B".into(), -1),
        ("M".into(), -1),
        ("P".into(), -1),
        ("C".into(), -1),
        ("F".into(), -2),
        ("H".into(), -2),
        ("V".into(), -2),
        ("W".into(), -2),
        ("Y".into(), -2),
        ("K".into(), -2),
        ("J".into(), -3),
        ("X".into(), -3),
        ("Q".into(), -4),
        ("Z".into(), -4),
        ("?".into(), -2),
    ])
}

// Strategy interface for pluggable AI implementations.
pub trait AiStrategy {
    fn id(&self) -> &'static str;
    fn best_move(&self, state: &GameState, rules: &dyn Rules, config: &AiConfig) -> Option<EvaluatedMove>;
}

// Public API remains stable: these call into the default strategy.
pub fn best_move_greedy(state: &GameState, rules: &impl Rules, config: &AiConfig) -> Option<EvaluatedMove> {
    greedy::best_move_default(state, rules, config)
}

pub fn best_move(state: &GameState, rules: &impl Rules, level: AiDifficulty) -> Option<EvaluatedMove> {
    let cfg = AiConfig::for_difficulty(level);
    best_move_greedy(state, rules, &cfg)
}
