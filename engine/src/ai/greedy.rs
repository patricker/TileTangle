use rand::{rngs::StdRng, Rng, SeedableRng};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::time::Instant;

use crate::{
    movegen::{generate_moves, CandidateMove},
    GameState, MoveDraft, Rules,
};

use super::{AiConfig, OpponentModel};
use super::types::EvaluatedMove;
use super::heuristics::evaluate_candidate_move;

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

fn best_move_inner<R: Rules>(
    state: &GameState,
    rules: &R,
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
                if rules.commit(&mut next_state, validated, &score).is_ok() {
                    // Apply opponent visibility model before simulating reply
                    if ctx.config.opponent_model == OpponentModel::BagSampling {
                        mask_opponent_rack_with_bag_sample(&mut next_state);
                    }
                    if let Some(reply) = best_move_inner(&next_state, rules, depth.saturating_sub(1), ctx) {
                        eval.total -= reply.total;
                    }
                }
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

pub(crate) struct GreedyAi;

pub(crate) fn best_move_default<R: Rules>(state: &GameState, rules: &R, config: &AiConfig) -> Option<EvaluatedMove> {
    let mut ctx = SearchContext::new(config);
    best_move_inner(state, rules, config.lookahead_depth, &mut ctx)
}

fn mask_opponent_rack_with_bag_sample(state: &mut GameState) {
    let pid = state.to_move.0;
    let rack_size = state.players[pid].rack_size().unwrap_or(7);
    // Replace rack with a fresh sample from the bag WITHOUT returning current tiles.
    // This models hidden information: AI samples from remaining bag only.
    state.players[pid].rack.tiles.clear();
    let mut drawn = state.bag.draw(rack_size);
    state.players[pid].rack.tiles.append(&mut drawn);
}
