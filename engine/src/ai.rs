use rand::{rngs::StdRng, Rng, SeedableRng};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::{
    geometry::{BoardGeometry, CellId},
    movegen::{generate_moves, CandidateMove},
    GameState, MoveDraft, Rules, Tile, Tileset,
};

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

fn tileset_symbol_for_kind(tileset: &Tileset, kind_id: &str) -> String {
    tileset
        .tile_kinds
        .iter()
        .find(|tk| tk.id == kind_id)
        .map(|tk| tk.symbol.clone())
        .unwrap_or_else(|| kind_id.to_string())
}

fn leftover_counts_from_rack(rack: &[String], placements: &[(CellId, Tile)]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for kid in rack { *counts.entry(kid.clone()).or_default() += 1; }
    for (_, tile) in placements {
        if let Some(entry) = counts.get_mut(&tile.kind_id) && *entry > 0 { *entry -= 1; }
    }
    counts.retain(|_, v| *v > 0);
    counts
}

fn rack_leave_score(config: &AiConfig, tileset: &Tileset, leftover: &HashMap<String, usize>) -> i32 {
    leftover
        .iter()
        .map(|(kid, count)| {
            let sym = tileset_symbol_for_kind(tileset, kid).to_uppercase();
            let val = config
                .rack_leave
                .get(&sym)
                .or_else(|| config.rack_leave.get(kid))
                .copied()
                .unwrap_or(0);
            val * (*count as i32)
        })
        .sum()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedMove {
    pub candidate: CandidateMove,
    pub rack_leave: i32,
    pub board_equity: i32,
    pub endgame_penalty: i32,
    pub total: i32,
}

fn board_equity_bonus(state: &GameState, candidate: &CandidateMove) -> i32 {
    let mut bonus = 0;
    let placed: HashSet<CellId> = candidate.placements.iter().map(|(cid, _)| *cid).collect();
    for (cid, _) in &candidate.placements {
        for neigh in state.board.geom.neighbors(*cid) {
            if placed.contains(&neigh) { continue; }
            if state.board.cells[neigh.0 as usize].stack.is_empty() { bonus += 1; }
        }
    }
    bonus
}

fn endgame_penalty(state: &GameState, leftover: &HashMap<String, usize>) -> i32 {
    if state.bag.remaining() > 0 { return 0; }
    let mut penalty = 0;
    for (kind_id, count) in leftover {
        if *count == 0 { continue; }
        if let Some(kind) = state.tileset.tile_kinds.iter().find(|tk| tk.id == *kind_id) {
            penalty -= (*count as i32) * (kind.score as i32);
        }
    }
    penalty
}

pub fn evaluate_candidate_move(
    state: &GameState,
    candidate: CandidateMove,
    rack: &[String],
    config: &AiConfig,
) -> EvaluatedMove {
    let leftover = leftover_counts_from_rack(rack, &candidate.placements);
    let leave_score = rack_leave_score(config, &state.tileset, &leftover);
    let board_eq = board_equity_bonus(state, &candidate);
    let end_pen = endgame_penalty(state, &leftover);
    let total = candidate.score + leave_score + board_eq + end_pen;
    EvaluatedMove { candidate, rack_leave: leave_score, board_equity: board_eq, endgame_penalty: end_pen, total }
}

pub fn best_move_greedy(
    state: &GameState,
    rules: &impl Rules,
    config: &AiConfig,
) -> Option<EvaluatedMove> {
    let mut ctx = SearchContext::new(config);
    best_move_inner(state, rules, config.lookahead_depth, &mut ctx)
}

pub fn best_move(state: &GameState, rules: &impl Rules, level: AiDifficulty) -> Option<EvaluatedMove> {
    let cfg = AiConfig::for_difficulty(level);
    best_move_greedy(state, rules, &cfg)
}

#[derive(Debug)]
struct BestCandidate { eval: EvaluatedMove, adjusted_total: i32 }

struct SearchContext<'a> {
    config: &'a AiConfig,
    rng: Option<StdRng>,
    nodes: usize,
    deadline: Option<Instant>,
}

impl<'a> SearchContext<'a> {
    fn new(config: &'a AiConfig) -> Self {
        let mut rng = config.randomness.map(StdRng::seed_from_u64);
        if rng.is_none() && config.requires_rng() { rng = Some(StdRng::from_entropy()); }
        Self { config, rng, nodes: 0, deadline: config.max_duration.map(|d| Instant::now().checked_add(d).unwrap_or(Instant::now())) }
    }
    fn rng_mut(&mut self) -> Option<&mut StdRng> {
        if self.rng.is_none() && (self.config.randomness.is_some() || self.config.requires_rng()) {
            self.rng = Some(match self.config.randomness { Some(seed) => StdRng::seed_from_u64(seed), None => StdRng::from_entropy() });
        }
        self.rng.as_mut()
    }
    fn record_node(&mut self) { self.nodes = self.nodes.saturating_add(1); }
    fn node_limit_hit(&self) -> bool { self.config.max_nodes.map(|limit| self.nodes >= limit).unwrap_or(false) }
    fn time_limit_hit(&self) -> bool { self.deadline.map(|deadline| Instant::now() >= deadline).unwrap_or(false) }
    fn sample_noise(&mut self) -> i32 { let range = self.config.noise_range; if range <= 0 { 0 } else { self.rng_mut().map(|rng| rng.gen_range(-range..=range)).unwrap_or(0) } }
    fn random_bool(&mut self, p: f64) -> bool { if p <= 0.0 { return false; } let p = p.min(1.0); self.rng_mut().map(|rng| rng.gen_bool(p)).unwrap_or(false) }
}

fn best_move_inner(
    state: &GameState,
    rules: &impl Rules,
    depth: usize,
    ctx: &mut SearchContext<'_>,
) -> Option<EvaluatedMove> {
    let pid = state.to_move.0;
    let rack: Vec<String> = state.players[pid].rack.tiles.iter().map(|t| t.kind_id.clone()).collect();
    let mut candidates = generate_moves(state, rules, &rack, ctx.config.max_move_len);
    if let Some(limit) = ctx.config.candidate_limit { if candidates.len() > limit { candidates.truncate(limit); } }
    else if ctx.config.reply_move_limit != usize::MAX && candidates.len() > ctx.config.reply_move_limit { candidates.truncate(ctx.config.reply_move_limit); }

    let mut best: Option<BestCandidate> = None;
    if ctx.config.parallel_eval && depth == ctx.config.lookahead_depth && ctx.config.lookahead_depth == 0 {
        #[cfg(feature = "parallel")]
        {
            let evals: Vec<(CandidateMove, EvaluatedMove)> = candidates
                .into_par_iter()
                .map(|cand| {
                    let eval = evaluate_candidate_move(state, cand.clone(), &rack, ctx.config);
                    (cand, eval)
                })
                .collect();
            for (_cand, eval) in evals {
                let adjusted = eval.total + ctx.sample_noise();
                match &mut best {
                    None => best = Some(BestCandidate { eval, adjusted_total: adjusted }),
                    Some(current) => {
                        if adjusted > current.adjusted_total { *current = BestCandidate { eval, adjusted_total: adjusted }; }
                    }
                }
            }
            return best.map(|b| b.eval);
        }
    }

    for cand in candidates {
        if ctx.node_limit_hit() && best.is_some() { break; }
        if ctx.time_limit_hit() && best.is_some() { break; }
        ctx.record_node();
        let mut eval = evaluate_candidate_move(state, cand.clone(), &rack, ctx.config);
        if depth > 0 && !ctx.node_limit_hit() && !ctx.time_limit_hit() {
            let draft = MoveDraft { placements: cand.placements.clone() };
            if let Ok(validated) = rules.validate(state, &draft) {
                let score = rules.score(state, &validated);
                let mut next_state = state.clone();
                if rules.commit(&mut next_state, validated, &score).is_ok() &&
                    let Some(reply) = best_move_inner(&next_state, rules, depth.saturating_sub(1), ctx)
                { eval.total -= reply.total; }
            }
        }
        let adjusted = eval.total + ctx.sample_noise();
        match &mut best {
            None => best = Some(BestCandidate { eval, adjusted_total: adjusted }),
            Some(current) => {
                let better = adjusted > current.adjusted_total
                    || (adjusted == current.adjusted_total && eval.total > current.eval.total)
                    || (adjusted == current.adjusted_total && eval.total == current.eval.total && eval.rack_leave > current.eval.rack_leave)
                    || (adjusted == current.adjusted_total && eval.total == current.eval.total && eval.rack_leave == current.eval.rack_leave && ctx.random_bool(0.5));
                if better { *current = BestCandidate { eval, adjusted_total: adjusted }; }
            }
        }
        if ctx.time_limit_hit() && best.is_some() { break; }
    }
    best.map(|b| b.eval)
}
