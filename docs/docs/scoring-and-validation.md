---
sidebar_position: 5
---

# Scoring & Validation (2D)

- Validation: straight-line placements, contiguous span (gaps allowed only if filled by existing tiles), first move must cover center, subsequent moves must touch existing.
- Bonuses: letter multipliers apply to newly placed tiles only; word multipliers multiply the entire word if the newly placed tile sits on that bonus.
- Free word mode: dictionary checks are disabled until Phase 3.
- Bingo: configurable bonus (default 50) applied when using full rack (heuristic in MVP).
 - Reading direction: LTR by default; RTL reverses horizontal word assembly for dictionary checks and scoring.
 - Stacking (Phase 9): optional overlay rules (max height; forbid same-symbol overlays). Scoring modes:
   - TopOnly: only the top tile contributes; letter multipliers apply to newly placed tiles only.
   - SumStack: sum all underlying tiles plus the top (with multipliers); word multipliers apply as usual.

Example

```
let rules = CrosswordRules::default();
let center = CrosswordRules::center_cell(&state.board.geom);
let mv1 = MoveDraft { placements: vec![(center, Tile { kind_id: "A".into(), mark: None })] };
let v1 = rules.validate(&state, &mv1)?;
let sc1 = rules.score(&state, &v1);
rules.commit(&mut state, v1, &sc1)?;
```

See example: `cargo run --example scoring_demo -p tiletangle-engine`.

Animated breakdown: See the companion page Animated Move Breakdown for a step-by-step visual of letter sums and word multipliers.
