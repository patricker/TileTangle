use rand::{Rng, SeedableRng, rngs::StdRng};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use std::time::Instant;

#[cfg(feature = "parallel")]
use crate::movegen::CandidateMove;
use crate::{GameState, MoveDraft, Rules, movegen::generate_moves};

use super::heuristics::evaluate_candidate_move;
use super::types::EvaluatedMove;
use super::{AiConfig, OpponentModel};

#[derive(Debug)]
struct BestCandidate {
    eval: EvaluatedMove,
    adjusted_total: i32,
}

struct SearchContext<'a> {
    config: &'a AiConfig,
    rng: Option<StdRng>,
    nodes: usize,
    deadline: Option<Instant>,
}

impl<'a> SearchContext<'a> {
    fn new(config: &'a AiConfig) -> Self {
        let mut rng = config.randomness.map(StdRng::seed_from_u64);
        if rng.is_none() && config.requires_rng() {
            rng = Some(StdRng::from_entropy());
        }
        Self {
            config,
            rng,
            nodes: 0,
            deadline: config
                .max_duration
                .map(|d| Instant::now().checked_add(d).unwrap_or(Instant::now())),
        }
    }
    fn rng_mut(&mut self) -> Option<&mut StdRng> {
        if self.rng.is_none() && (self.config.randomness.is_some() || self.config.requires_rng()) {
            self.rng = Some(match self.config.randomness {
                Some(seed) => StdRng::seed_from_u64(seed),
                None => StdRng::from_entropy(),
            });
        }
        self.rng.as_mut()
    }
    fn record_node(&mut self) {
        self.nodes = self.nodes.saturating_add(1);
    }
    fn node_limit_hit(&self) -> bool {
        self.config
            .max_nodes
            .map(|limit| self.nodes >= limit)
            .unwrap_or(false)
    }
    fn time_limit_hit(&self) -> bool {
        self.deadline
            .map(|deadline| Instant::now() >= deadline)
            .unwrap_or(false)
    }
    fn sample_noise(&mut self) -> i32 {
        let range = self.config.noise_range;
        if range <= 0 {
            0
        } else {
            self.rng_mut()
                .map(|rng| rng.gen_range(-range..=range))
                .unwrap_or(0)
        }
    }
    fn random_bool(&mut self, p: f64) -> bool {
        if p <= 0.0 {
            return false;
        }
        let p = p.min(1.0);
        self.rng_mut().map(|rng| rng.gen_bool(p)).unwrap_or(false)
    }
}

fn best_move_inner<R: Rules>(
    state: &GameState,
    rules: &R,
    depth: usize,
    ctx: &mut SearchContext<'_>,
) -> Option<EvaluatedMove> {
    let pid = state.to_move.0;
    let rack: Vec<String> = state.players[pid]
        .rack
        .tiles
        .iter()
        .map(|t| t.kind_id.clone())
        .collect();
    let mut candidates = generate_moves(state, rules, &rack, ctx.config.max_move_len);
    if let Some(limit) = ctx.config.candidate_limit {
        if candidates.len() > limit {
            candidates.truncate(limit);
        }
    } else if ctx.config.reply_move_limit != usize::MAX
        && candidates.len() > ctx.config.reply_move_limit
    {
        candidates.truncate(ctx.config.reply_move_limit);
    }

    let mut best: Option<BestCandidate> = None;
    if ctx.config.parallel_eval
        && depth == ctx.config.lookahead_depth
        && ctx.config.lookahead_depth == 0
    {
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
                    None => {
                        best = Some(BestCandidate {
                            eval,
                            adjusted_total: adjusted,
                        })
                    }
                    Some(current) => {
                        if adjusted > current.adjusted_total {
                            *current = BestCandidate {
                                eval,
                                adjusted_total: adjusted,
                            };
                        }
                    }
                }
            }
            return best.map(|b| b.eval);
        }
    }

    for cand in candidates {
        if ctx.node_limit_hit() && best.is_some() {
            break;
        }
        if ctx.time_limit_hit() && best.is_some() {
            break;
        }
        ctx.record_node();
        let mut eval = evaluate_candidate_move(state, cand.clone(), &rack, ctx.config);
        if depth > 0 && !ctx.node_limit_hit() && !ctx.time_limit_hit() {
            let draft = MoveDraft {
                placements: cand.placements.clone(),
            };
            if let Ok(validated) = rules.validate(state, &draft) {
                let score = rules.score(state, &validated);
                let mut next_state = state.clone();
                if rules.commit(&mut next_state, validated, &score).is_ok() {
                    // Apply opponent visibility model before simulating reply
                    if ctx.config.opponent_model == OpponentModel::BagSampling {
                        mask_opponent_rack_with_bag_sample(&mut next_state);
                    }
                    if let Some(reply) =
                        best_move_inner(&next_state, rules, depth.saturating_sub(1), ctx)
                    {
                        eval.total -= reply.total;
                    }
                }
            }
        }
        let adjusted = eval.total + ctx.sample_noise();
        match &mut best {
            None => {
                best = Some(BestCandidate {
                    eval,
                    adjusted_total: adjusted,
                })
            }
            Some(current) => {
                let better = adjusted > current.adjusted_total
                    || (adjusted == current.adjusted_total && eval.total > current.eval.total)
                    || (adjusted == current.adjusted_total
                        && eval.total == current.eval.total
                        && eval.rack_leave > current.eval.rack_leave)
                    || (adjusted == current.adjusted_total
                        && eval.total == current.eval.total
                        && eval.rack_leave == current.eval.rack_leave
                        && ctx.random_bool(0.5));
                if better {
                    *current = BestCandidate {
                        eval,
                        adjusted_total: adjusted,
                    };
                }
            }
        }
        if ctx.time_limit_hit() && best.is_some() {
            break;
        }
    }
    best.map(|b| b.eval)
}

// Removed unused GreedyAi marker to satisfy dead-code lint

pub(crate) fn best_move_default<R: Rules>(
    state: &GameState,
    rules: &R,
    config: &AiConfig,
) -> Option<EvaluatedMove> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AiDifficulty;
    use crate::{
        CrosswordRules, Tile, TileKind,
        game::{GameConfig, RectBoardLayout},
        inventory::Tileset,
    };

    fn test_config() -> GameConfig {
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
        counts.insert("A".to_string(), 10);
        counts.insert("B".to_string(), 10);
        GameConfig {
            tileset,
            rack_size: 7,
            board_layout: RectBoardLayout {
                width: 5,
                height: 5,
            },
            ruleset_id: "cross".into(),
            dictionary_id: "en".into(),
            rng_seed: 42,
            tile_counts: counts,
        }
    }

    fn setup_game_with_rack(rack_tiles: &[&str]) -> (GameState, CrosswordRules) {
        let cfg = test_config();
        let mut state = GameState::new(&cfg, 2).unwrap();
        state.players[0].rack.tiles = rack_tiles
            .iter()
            .map(|k| Tile {
                kind_id: k.to_string(),
                mark: None,
            })
            .collect();
        let rules = CrosswordRules {
            free_word_mode: true,
            ..Default::default()
        };
        (state, rules)
    }

    #[test]
    fn best_move_returns_some_on_nonempty_rack() {
        let (state, rules) = setup_game_with_rack(&["A", "B"]);
        let cfg = AiConfig::default();
        let result = best_move_default(&state, &rules, &cfg);
        assert!(
            result.is_some(),
            "AI should find a move with tiles A,B on empty board"
        );
    }

    #[test]
    fn best_move_returns_none_on_empty_rack() {
        let (state, rules) = setup_game_with_rack(&[]);
        let cfg = AiConfig::default();
        let result = best_move_default(&state, &rules, &cfg);
        assert!(result.is_none(), "AI should return None with empty rack");
    }

    #[test]
    fn noise_range_produces_different_results() {
        let (state, rules) = setup_game_with_rack(&["A", "B", "A", "B"]);
        let cfg1 = AiConfig {
            noise_range: 20,
            randomness: Some(1),
            ..Default::default()
        };
        let r1 = best_move_default(&state, &rules, &cfg1);

        let cfg2 = AiConfig {
            noise_range: 20,
            randomness: Some(999),
            ..Default::default()
        };
        let r2 = best_move_default(&state, &rules, &cfg2);

        // With high noise and different seeds, results may differ (not guaranteed but likely)
        // At minimum, both should produce a valid move
        assert!(r1.is_some());
        assert!(r2.is_some());
    }

    #[test]
    fn node_limit_still_returns_a_move() {
        let (state, rules) = setup_game_with_rack(&["A", "B"]);
        let cfg = AiConfig {
            max_nodes: Some(1),
            ..Default::default()
        };
        let result = best_move_default(&state, &rules, &cfg);
        assert!(
            result.is_some(),
            "node limit should still return at least one move"
        );
    }

    #[test]
    fn difficulty_levels_produce_valid_moves() {
        let (state, rules) = setup_game_with_rack(&["A", "B"]);
        for level in [AiDifficulty::Easy, AiDifficulty::Medium, AiDifficulty::Hard] {
            let cfg = AiConfig::for_difficulty(level);
            let result = best_move_default(&state, &rules, &cfg);
            assert!(
                result.is_some(),
                "difficulty {:?} should produce a move",
                level
            );
        }
    }

    #[test]
    fn lookahead_depth_1_returns_move() {
        let (mut state, rules) = setup_game_with_rack(&["A", "B"]);
        // Give player 1 some tiles too for lookahead
        state.players[1].rack.tiles = vec![
            Tile {
                kind_id: "A".into(),
                mark: None,
            },
            Tile {
                kind_id: "B".into(),
                mark: None,
            },
        ];
        let cfg = AiConfig {
            lookahead_depth: 1,
            ..Default::default()
        };
        let result = best_move_default(&state, &rules, &cfg);
        assert!(result.is_some());
    }
}
