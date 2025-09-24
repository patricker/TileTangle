use crate::{
    board::Board,
    geometry::{BoardGeometry, CellId, Coord2D, RectGridGeometry},
    inventory::Tileset,
    EngineError, GameState, MoveDraft, Tile, TileKind,
};

use std::collections::{HashMap, HashSet};

#[cfg(feature = "simd")]
use wide::i32x4;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScoreBreakdown {
    pub total: i32,
    pub main_word: String,
    pub main_score: i32,
    pub cross_words: Vec<(String, i32)>,
    pub bingo: bool,
}

pub trait Rules {
    fn validate(&self, state: &GameState, draft: &MoveDraft) -> Result<ValidatedMove, EngineError>;
    fn score(&self, state: &GameState, mv: &ValidatedMove) -> ScoreBreakdown;
    fn commit(
        &self,
        state: &mut GameState,
        mv: ValidatedMove,
        score: &ScoreBreakdown,
    ) -> Result<(), EngineError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadingDirection {
    #[default]
    LTR,
    RTL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackScoring {
    #[default]
    TopOnly,
    SumStack,
}

#[derive(Debug, Clone)]
pub struct CrosswordRules {
    pub free_word_mode: bool,
    pub bingo_bonus: i32,
    pub require_center_first_move: bool,
    pub reading_dir: ReadingDirection,
    pub stacking_enabled: bool,
    pub stacking_max_height: usize,
    pub forbid_same_symbol_overlay: bool,
    pub stacking_scoring: StackScoring,
}

impl Default for CrosswordRules {
    fn default() -> Self {
        Self {
            free_word_mode: true,
            bingo_bonus: 50,
            require_center_first_move: true,
            reading_dir: ReadingDirection::LTR,
            stacking_enabled: false,
            stacking_max_height: 7,
            forbid_same_symbol_overlay: true,
            stacking_scoring: StackScoring::TopOnly,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedMove {
    pub placements: Vec<(CellId, Tile)>,
    pub line_is_row: bool,
}

impl CrosswordRules {
    pub fn center_cell(geom: &RectGridGeometry) -> CellId {
        let cx = (geom.width / 2) as i32;
        let cy = (geom.height / 2) as i32;
        geom.to_cell_id(Coord2D { x: cx, y: cy }).unwrap()
    }

    pub(crate) fn cell_has_tile(board: &Board<RectGridGeometry>, id: CellId) -> bool {
        !board.cells[id.0 as usize].stack.is_empty()
    }

    fn line_contiguous(board: &Board<RectGridGeometry>, ids: &[CellId], is_row: bool) -> bool {
        if ids.is_empty() {
            return false;
        }
        let mut coords: Vec<Coord2D> = ids
            .iter()
            .map(|id| board.geom.from_cell_id(*id).unwrap())
            .collect();
        coords.sort_by(|a, b| if is_row { a.x.cmp(&b.x) } else { a.y.cmp(&b.y) });
        let fixed = if is_row { coords[0].y } else { coords[0].x };
        let (min, max) = if is_row {
            (coords.first().unwrap().x, coords.last().unwrap().x)
        } else {
            (coords.first().unwrap().y, coords.last().unwrap().y)
        };
        for k in min..=max {
            let c = if is_row {
                Coord2D { x: k, y: fixed }
            } else {
                Coord2D { x: fixed, y: k }
            };
            let id = board.geom.to_cell_id(c).unwrap();
            if !Self::cell_has_tile(board, id) && !ids.contains(&id) {
                return false;
            }
        }
        true
    }

    pub(crate) fn adjacent_to_existing(board: &Board<RectGridGeometry>, ids: &[CellId]) -> bool {
        for id in ids {
            if !board.cells[id.0 as usize].stack.is_empty() {
                return true;
            }
            for n in board.geom.neighbors(*id) {
                if Self::cell_has_tile(board, n) {
                    return true;
                }
            }
        }
        false
    }

    fn tileset_lookup_kind<'a>(tileset: &'a Tileset, kind_id: &str) -> Option<&'a TileKind> {
        tileset.tile_kinds.iter().find(|tk| tk.id == kind_id)
    }

    pub(crate) fn tile_symbol_and_score(tileset: &Tileset, tile: &Tile) -> (i16, String) {
        if let Some(tk) = Self::tileset_lookup_kind(tileset, &tile.kind_id) {
            if tk.is_blank {
                let sym = tile.mark.clone().unwrap_or_else(|| tk.symbol.clone());
                (tk.score, sym)
            } else {
                (tk.score, tk.symbol.clone())
            }
        } else {
            (0, "?".into())
        }
    }

    fn form_word(
        board: &Board<RectGridGeometry>,
        tileset: &Tileset,
        start: Coord2D,
        dir: (i32, i32),
        placed: &HashSet<CellId>,
        reverse_horiz: bool,
        stack_mode: StackScoring,
    ) -> (String, i32, usize) {
        // move to beginning
        let mut c = start;
        loop {
            let prev = Coord2D { x: c.x - dir.0, y: c.y - dir.1 };
            if let Some(id) = board.geom.to_cell_id(prev)
                && Self::cell_has_tile(board, id)
            {
                c = prev;
                continue;
            }
            break;
        }
        let mut tokens: Vec<String> = Vec::new();
        let mut score: i32 = 0;
        let mut word_mul: i32 = 1;
        let mut tiles_count: usize = 0;
        loop {
            if let Some(id) = board.geom.to_cell_id(c)
                && Self::cell_has_tile(board, id)
            {
                let cell = &board.cells[id.0 as usize];
                let tile = cell.stack.last().unwrap();
                let (ls, sym) = Self::tile_symbol_and_score(tileset, tile);
                tokens.push(sym);
                let mut letter_mul = 1i32;
                if placed.contains(&id)
                    && let Some(b) = board.bonuses.get(&id)
                {
                    letter_mul = b.letter_mul as i32;
                    word_mul *= b.word_mul as i32;
                }
                let add = match stack_mode {
                    StackScoring::TopOnly => (ls as i32) * letter_mul,
                    StackScoring::SumStack => {
                        let mut sum_under = 0i32;
                        if !board.cells[id.0 as usize].stack.is_empty() {
                            for t in board.cells[id.0 as usize]
                                .stack
                                .iter()
                                .take(board.cells[id.0 as usize].stack.len().saturating_sub(1))
                            {
                                let (s_u, _) = Self::tile_symbol_and_score(tileset, t);
                                sum_under += s_u as i32;
                            }
                        }
                        sum_under + (ls as i32) * letter_mul
                    }
                };
                score += add;
                tiles_count += 1;
                c = Coord2D { x: c.x + dir.0, y: c.y + dir.1 };
                continue;
            }
            break;
        }
        let word = if reverse_horiz {
            tokens.into_iter().rev().collect::<Vec<_>>().join("")
        } else {
            tokens.join("")
        };
        (word, score * word_mul, tiles_count)
    }

    fn graph_adjacent_to_existing(board: &Board<RectGridGeometry>, ids: &HashSet<CellId>) -> bool {
        for id in ids {
            if !board.cells[id.0 as usize].stack.is_empty() {
                return true;
            }
            for n in board.geom.neighbors(*id) {
                if !ids.contains(&n) && Self::cell_has_tile(board, n) {
                    return true;
                }
            }
        }
        false
    }

    fn graph_find_main_dir_and_path(
        board: &Board<RectGridGeometry>,
        placed: &HashSet<CellId>,
    ) -> Option<(String, Vec<CellId>)> {
        if placed.len() == 1 {
            let id = *placed.iter().next().unwrap();
            let tag = board
                .geom
                .neighbors_with_tags(id)
                .first()
                .map(|(_, t)| t.to_string())
                .unwrap_or_else(|| "E".into());
            return Some((tag, vec![id]));
        }
        let mut tags: HashSet<String> = HashSet::new();
        for &id in placed.iter() {
            for (n, t) in board.geom.neighbors_with_tags(id) {
                if placed.contains(&n) || Self::cell_has_tile(board, n) {
                    tags.insert(t.to_string());
                }
            }
        }
        for tag in tags.into_iter() {
            let mut deg: HashMap<CellId, usize> = HashMap::new();
            for &u in placed.iter() {
                let mut d = 0usize;
                for (v, t) in board.geom.neighbors_with_tags(u) {
                    if t == tag && placed.contains(&v) {
                        d += 1;
                    }
                }
                deg.insert(u, d);
            }
            let endpoints: Vec<CellId> = deg
                .iter()
                .filter_map(|(k, d)| if *d <= 1 { Some(*k) } else { None })
                .collect();
            if endpoints.is_empty() || endpoints.len() > 2 {
                continue;
            }
            let start = endpoints[0];
            let goal = if endpoints.len() == 2 { endpoints[1] } else { start };
            if let Some(path) = bfs_path_on_dir(board, start, goal, &tag, placed) {
                return Some((tag, path));
            }
        }
        None
    }
}

impl Rules for CrosswordRules {
    fn validate(&self, state: &GameState, draft: &MoveDraft) -> Result<ValidatedMove, EngineError> {
        if draft.placements.is_empty() {
            return Err(EngineError::Config("no tiles placed"));
        }
        if state.board.geom.has_graph() {
            for (cid, tile) in &draft.placements {
                let occupied = CrosswordRules::cell_has_tile(&state.board, *cid);
                if occupied {
                    if !self.stacking_enabled {
                        return Err(EngineError::Collision(*cid));
                    }
                    let cell = &state.board.cells[cid.0 as usize];
                    if cell.stack.len() + 1 > self.stacking_max_height {
                        return Err(EngineError::Config("stack too high"));
                    }
                    if self.forbid_same_symbol_overlay
                        && let Some(top) = cell.stack.last()
                    {
                        let (_, top_sym) = Self::tile_symbol_and_score(&state.tileset, top);
                        let (_, new_sym) = Self::tile_symbol_and_score(&state.tileset, tile);
                        if top_sym == new_sym {
                            return Err(EngineError::Config("cannot overlay same symbol"));
                        }
                    }
                }
            }
            let ids_set: HashSet<CellId> = draft.placements.iter().map(|(id, _)| *id).collect();
            let any_on_board = state.board.cells.iter().any(|c| !c.stack.is_empty());
            if any_on_board && !Self::graph_adjacent_to_existing(&state.board, &ids_set) {
                return Err(EngineError::Config("must connect to existing tiles"));
            }
            if Self::graph_find_main_dir_and_path(&state.board, &ids_set).is_none() {
                return Err(EngineError::Config("must be straight line"));
            }
            return Ok(ValidatedMove { placements: draft.placements.clone(), line_is_row: true });
        }

        for (cid, tile) in &draft.placements {
            let occupied = Self::cell_has_tile(&state.board, *cid);
            if occupied {
                if !self.stacking_enabled {
                    return Err(EngineError::Collision(*cid));
                }
                let cell = &state.board.cells[cid.0 as usize];
                if cell.stack.len() + 1 > self.stacking_max_height {
                    return Err(EngineError::Config("stack too high"));
                }
                if self.forbid_same_symbol_overlay
                    && let Some(top) = cell.stack.last()
                {
                    let (_, top_sym) = Self::tile_symbol_and_score(&state.tileset, top);
                    let (_, new_sym) = Self::tile_symbol_and_score(&state.tileset, tile);
                    if top_sym == new_sym {
                        return Err(EngineError::Config("cannot overlay same symbol"));
                    }
                }
            }
        }
        let coords: Vec<Coord2D> = draft
            .placements
            .iter()
            .map(|(id, _)| state.board.geom.from_cell_id(*id).unwrap())
            .collect();
        let same_row = coords.iter().all(|c| c.y == coords[0].y);
        let same_col = coords.iter().all(|c| c.x == coords[0].x);
        if !same_row && !same_col { return Err(EngineError::Config("must be straight line")); }
        let is_row = same_row;
        let ids: Vec<CellId> = draft.placements.iter().map(|(id, _)| *id).collect();
        if !Self::line_contiguous(&state.board, &ids, is_row) {
            return Err(EngineError::Config("not contiguous"));
        }
        let any_on_board = state.board.cells.iter().any(|c| !c.stack.is_empty());
        if !any_on_board {
            if self.require_center_first_move {
                let center = Self::center_cell(&state.board.geom);
                if !ids.contains(&center) { return Err(EngineError::Config("first move must cover center")); }
            }
        } else if !Self::adjacent_to_existing(&state.board, &ids) {
            return Err(EngineError::Config("must connect to existing tiles"));
        }
        Ok(ValidatedMove { placements: draft.placements.clone(), line_is_row: is_row })
    }

    fn score(&self, state: &GameState, mv: &ValidatedMove) -> ScoreBreakdown {
        if state.board.geom.has_graph() {
            let placed_ids: HashSet<CellId> = mv.placements.iter().map(|(id, _)| *id).collect();
            let mut temp_board = state.board.clone();
            for (cid, tile) in &mv.placements { temp_board.cells[cid.0 as usize].stack.push(tile.clone()); }
            let mut main_word = String::new();
            let mut main_score = 0;
            let mut cross_words: Vec<(String, i32)> = Vec::new();
            if let Some((tag, path)) = Self::graph_find_main_dir_and_path(&state.board, &placed_ids) {
                let (w, s) = score_word_on_path(&temp_board, &state.tileset, &path, &placed_ids, self.stacking_scoring);
                main_word = w;
                main_score = s;
                for (cid, _) in &mv.placements {
                    let mut seen: HashSet<String> = HashSet::new();
                    for (_, t) in state.board.geom.neighbors_with_tags(*cid) {
                        if t == tag { continue; }
                        if seen.insert(t.to_string()) {
                            let line = collect_line_on_dir(&temp_board, *cid, t, &placed_ids);
                            if line.len() > 1 {
                                let (cw, cs) = score_word_on_path(&temp_board, &state.tileset, &line, &placed_ids, self.stacking_scoring);
                                cross_words.push((cw, cs));
                            }
                        }
                    }
                }
            }
            let mut total = main_score;
            for (_, s) in &cross_words { total += *s; }

            if !self.free_word_mode {
                if let Some(dict) = &state.dictionary {
                    if !dict.contains(&main_word) {
                        return ScoreBreakdown { total: -1, main_word, main_score: -1, cross_words: vec![], bingo: false };
                    }
                    for (w, _) in &cross_words {
                        if !dict.contains(w) {
                            return ScoreBreakdown { total: -1, main_word, main_score: -1, cross_words: vec![], bingo: false };
                        }
                    }
                }
            }

            let bingo = mv.placements.len() >= state.players[state.to_move.0].rack.tiles.len()
                && !mv.placements.is_empty()
                && state.players[state.to_move.0].rack.len() >= 7;
            let bingo = if bingo { total += self.bingo_bonus; true } else { false };
            return ScoreBreakdown { total, main_word, main_score, cross_words, bingo };
        }

        // Non-graph boards: compute main word and cross-words using orthogonal traversal
        let mut placed_ids: HashSet<CellId> = HashSet::new();
        for (cid, _) in &mv.placements { placed_ids.insert(*cid); }
        let mut total = 0;
        let mut main_word = String::new();
        let mut main_score = 0;
        let mut cross_words: Vec<(String, i32)> = Vec::new();
        let dir = if mv.line_is_row { (1, 0) } else { (0, 1) };
        let pdir = if mv.line_is_row { (0, 1) } else { (1, 0) };
        let pid = state.to_move.0;
        let reverse_main = self.reading_dir == ReadingDirection::RTL && dir.1 == 0;
        let reverse_cross = self.reading_dir == ReadingDirection::RTL && pdir.1 == 0;

        // Build overlay: push tiles temporarily on a cloned board for scoring
        let mut temp_board = state.board.clone();
        for (cid, tile) in &mv.placements { temp_board.cells[cid.0 as usize].stack.push(tile.clone()); }

        // Find main word by scanning from the first placed cell along `dir`
        if let Some((start_id, _)) = mv.placements.first() {
            let start = temp_board.geom.from_cell_id(*start_id).unwrap();
            let (w, s, _count) = Self::form_word(&temp_board, &state.tileset, start, dir, &placed_ids, reverse_main, self.stacking_scoring);
            main_word = w;
            main_score = s;
        }

        // Cross words at each newly placed tile
        for (cid, _) in &mv.placements {
            let start = temp_board.geom.from_cell_id(*cid).unwrap();
            let (w, s, count) = Self::form_word(&temp_board, &state.tileset, start, pdir, &placed_ids, reverse_cross, self.stacking_scoring);
            if count > 1 { cross_words.push((w, s)); }
        }
        total += main_score + cross_words.iter().map(|(_, s)| *s).sum::<i32>();

        if !self.free_word_mode {
            if let Some(dict) = &state.dictionary {
                if !dict.contains(&main_word) {
                    return ScoreBreakdown { total: -1, main_word, main_score: -1, cross_words: vec![], bingo: false };
                }
                for (w, _) in &cross_words {
                    if !dict.contains(w) {
                        return ScoreBreakdown { total: -1, main_word, main_score: -1, cross_words: vec![], bingo: false };
                    }
                }
            }
        }

        let bingo = mv.placements.len() >= state.players[pid].rack.tiles.len()
            && !mv.placements.is_empty()
            && state.players[pid].rack.len() >= 7;
        let bingo = if bingo { total += self.bingo_bonus; true } else { false };
        ScoreBreakdown { total, main_word, main_score, cross_words, bingo }
    }

    fn commit(
        &self,
        state: &mut GameState,
        mv: ValidatedMove,
        score: &ScoreBreakdown,
    ) -> Result<(), EngineError> {
        for (cid, tile) in &mv.placements { state.board.cells[cid.0 as usize].stack.push(tile.clone()); }
        let pid = state.to_move.0;
        state.players[pid].score += score.total;
        let mut used_counts: HashMap<String, usize> = HashMap::new();
        for (_, t) in &mv.placements { *used_counts.entry(t.kind_id.clone()).or_default() += 1; }
        let mut new_rack = Vec::new();
        for t in state.players[pid].rack.tiles.drain(..) {
            if let Some(entry) = used_counts.get_mut(&t.kind_id) && *entry > 0 {
                *entry -= 1; continue;
            }
            new_rack.push(t);
        }
        state.players[pid].rack.tiles = new_rack;
        let want = state.players[pid].rack.tiles.len();
        let draw_n = (state.players[pid].rack_size().unwrap_or(7)).saturating_sub(want);
        let mut drawn = state.bag.draw(draw_n);
        let drawn_clone = drawn.clone();
        state.players[pid].rack.tiles.append(&mut drawn);

        state.push_event(
            pid,
            crate::GameEventKind::Play { placements: mv.placements.clone(), score: score.main_score, total: score.total },
        );
        state.log_draw(pid, &drawn_clone);
        state.advance_turn();
        Ok(())
    }
}

// ---- Graph helpers ----

fn bfs_path_on_dir(
    board: &Board<RectGridGeometry>,
    start: CellId,
    goal: CellId,
    tag: &str,
    placed: &HashSet<CellId>,
) -> Option<Vec<CellId>> {
    use std::collections::VecDeque;
    let mut q = VecDeque::new();
    let mut prev: HashMap<CellId, Option<CellId>> = HashMap::new();
    let mut seen: HashSet<CellId> = HashSet::new();
    q.push_back(start);
    seen.insert(start);
    prev.insert(start, None);
    while let Some(u) = q.pop_front() {
        if u == goal { break; }
        for (v, t) in board.geom.neighbors_with_tags(u) {
            if t != tag { continue; }
            if !placed.contains(&v) && board.cells[v.0 as usize].stack.is_empty() { continue; }
            if seen.insert(v) {
                prev.insert(v, Some(u));
                q.push_back(v);
            }
        }
    }
    if !prev.contains_key(&goal) { return None; }
    let mut path = Vec::new();
    let mut cur = goal;
    path.push(cur);
    while let Some(Some(p)) = prev.get(&cur) { cur = *p; path.push(cur); }
    path.reverse();
    Some(path)
}

fn collect_line_on_dir(
    board: &Board<RectGridGeometry>,
    center: CellId,
    tag: &str,
    placed: &HashSet<CellId>,
) -> Vec<CellId> {
    let neighs: Vec<CellId> = board
        .geom
        .neighbors_with_tags(center)
        .into_iter()
        .filter(|(_, t)| *t == tag)
        .map(|(n, _)| n)
        .collect();
    let mut back = center;
    if let Some(nb) = neighs.first() {
        let mut prev = center;
        let mut cur = *nb;
        loop {
            if !placed.contains(&cur) && board.cells[cur.0 as usize].stack.is_empty() { break; }
            let nxt = board
                .geom
                .neighbors_with_tags(cur)
                .into_iter()
                .filter(|(_, t)| *t == tag)
                .map(|(n, _)| n)
                .find(|n| *n != prev);
            back = cur;
            if let Some(n2) = nxt { prev = cur; cur = n2; } else { break; }
        }
    }
    let mut out = Vec::new();
    let mut prev = None;
    let mut cur = back;
    loop {
        if !placed.contains(&cur) && board.cells[cur.0 as usize].stack.is_empty() { break; }
        out.push(cur);
        let nxt = board
            .geom
            .neighbors_with_tags(cur)
            .into_iter()
            .filter(|(_, t)| *t == tag)
            .map(|(n, _)| n)
            .find(|n| Some(*n) != prev);
        if let Some(n2) = nxt { prev = Some(cur); cur = n2; } else { break; }
    }
    out
}

fn score_word_on_path(
    board: &Board<RectGridGeometry>,
    tileset: &Tileset,
    path: &[CellId],
    placed: &HashSet<CellId>,
    stack_mode: StackScoring,
) -> (String, i32) {
    let mut tokens: Vec<String> = Vec::new();
    let mut scalar_score: i32 = 0;
    let mut word_mul: i32 = 1;
    #[cfg(feature = "simd")]
    let mut top_only_scores: Vec<i32> = Vec::with_capacity(path.len());
    #[cfg(feature = "simd")]
    let mut top_only_multipliers: Vec<i32> = Vec::with_capacity(path.len());
    for id in path {
        let cell = &board.cells[id.0 as usize];
        if let Some(tile) = cell.stack.last() {
            let (ls, sym) = CrosswordRules::tile_symbol_and_score(tileset, tile);
            tokens.push(sym);
            let mut letter_mul = 1i32;
            if placed.contains(id)
                && let Some(b) = board.bonuses.get(id)
            {
                letter_mul = b.letter_mul as i32;
                word_mul *= b.word_mul as i32;
            }
            match stack_mode {
                StackScoring::TopOnly => {
                    #[cfg(feature = "simd")]
                    {
                        top_only_scores.push(ls as i32);
                        top_only_multipliers.push(letter_mul);
                    }
                    #[cfg(not(feature = "simd"))]
                    {
                        scalar_score += (ls as i32) * letter_mul;
                    }
                }
                StackScoring::SumStack => {
                    let mut sum_under = 0i32;
                    if !board.cells[id.0 as usize].stack.is_empty() {
                        for t in board.cells[id.0 as usize]
                            .stack
                            .iter()
                            .take(board.cells[id.0 as usize].stack.len().saturating_sub(1))
                        {
                            let (s_u, _) = CrosswordRules::tile_symbol_and_score(tileset, t);
                            sum_under += s_u as i32;
                        }
                    }
                    scalar_score += sum_under + (ls as i32) * letter_mul;
                }
            }
        }
    }
    let score = match stack_mode {
        StackScoring::TopOnly => {
            #[cfg(feature = "simd")]
            { simd_dot_product(&top_only_scores, &top_only_multipliers) }
            #[cfg(not(feature = "simd"))]
            { scalar_score }
        }
        StackScoring::SumStack => scalar_score,
    };
    (tokens.join(""), score * word_mul)
}

#[cfg(feature = "simd")]
fn simd_dot_product(lhs: &[i32], rhs: &[i32]) -> i32 {
    debug_assert_eq!(lhs.len(), rhs.len());
    let len = lhs.len().min(rhs.len());
    let mut total = 0i32;
    let mut i = 0usize;
    while i + 4 <= len {
        let a = i32x4::from([lhs[i], lhs[i + 1], lhs[i + 2], lhs[i + 3]]);
        let b = i32x4::from([rhs[i], rhs[i + 1], rhs[i + 2], rhs[i + 3]]);
        let prod = (a * b).to_array();
        total += prod[0] + prod[1] + prod[2] + prod[3];
        i += 4;
    }
    while i < len { total += lhs[i] * rhs[i]; i += 1; }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dict::{FstDictionary, DictionaryOptions},
        game::{GameConfig, RectBoardLayout},
        inventory::Tileset,
        text::NormalizationMode,
        Bonus, Coord2D, TileKind, Tile, TokenizerRef,
    };
    use std::collections::{HashMap, BTreeSet};

    #[test]
    fn scoring_with_bonuses_and_cross() {
        let tileset = Tileset { tile_kinds: vec![
            TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
        ]};
        let mut counts = HashMap::new(); counts.insert("A".to_string(), 10); counts.insert("B".to_string(), 10);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 3, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let rules = CrosswordRules::default();
        let center = CrosswordRules::center_cell(&st.board.geom);
        st.board.bonuses.insert(center, Bonus { letter_mul: 1, word_mul: 2, tags: BTreeSet::new() });
        let right = st.board.geom.to_cell_id(Coord2D { x: st.board.geom.from_cell_id(center).unwrap().x + 1, y: st.board.geom.from_cell_id(center).unwrap().y }).unwrap();
        st.board.bonuses.insert(right, Bonus { letter_mul: 3, word_mul: 1, tags: BTreeSet::new() });
        let mv1 = MoveDraft { placements: vec![(center, Tile { kind_id: "A".into(), mark: None })] };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert_eq!(sc1.total, 2);
        rules.commit(&mut st, v1, &sc1).unwrap();
        let mv2 = MoveDraft { placements: vec![(right, Tile { kind_id: "B".into(), mark: None })] };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.main_score >= 10);
    }

    #[test]
    fn rules_with_fst_dictionary() {
        let tileset = Tileset { tile_kinds: vec![
            TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
        ]};
        let mut counts = HashMap::new(); counts.insert("A".to_string(), 10); counts.insert("B".to_string(), 10);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 5, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(vec!["AB".to_string(), "B".to_string()], true)));
        let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft { placements: vec![(c, Tile { kind_id: "A".into(), mark: None })] };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert!(sc1.main_score < 0);
    }

    #[test]
    fn nfkc_move_accepts_halfwidth() {
        let tileset = Tileset { tile_kinds: vec![TileKind { id: "HALFPA".into(), symbol: "ﾊﾟ".into(), score: 3, is_blank: false, aliases: vec![] }] };
        let mut counts = HashMap::new(); counts.insert("HALFPA".to_string(), 5);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "jp".into(), rng_seed: 3, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let opts = DictionaryOptions { case_fold: false, min_len: None, max_len: None, norm: NormalizationMode::NFKC, tokenizer: TokenizerRef::default() };
        st.dictionary = Some(Box::new(FstDictionary::from_words_opts(vec!["パ".to_string()], opts)));
        let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
        let center = CrosswordRules::center_cell(&st.board.geom);
        let mv = MoveDraft { placements: vec![(center, Tile { kind_id: "HALFPA".into(), mark: None })] };
        let validated = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &validated);
        assert!(sc.main_score > 0);
        assert_eq!(sc.main_word, "ﾊﾟ");
    }

    #[test]
    fn rtl_hebrew_word_validates() {
        let tileset = Tileset { tile_kinds: vec![
            TileKind { id: "SHIN".into(), symbol: "ש".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "LAMED".into(), symbol: "ל".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "VAV".into(), symbol: "ו".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "MEMF".into(), symbol: "ם".into(), score: 1, is_blank: false, aliases: vec![] },
        ]};
        let mut counts = HashMap::new(); counts.insert("SHIN".to_string(), 4); counts.insert("LAMED".to_string(), 4); counts.insert("VAV".to_string(), 4); counts.insert("MEMF".to_string(), 4);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 7, height: 7 }, ruleset_id: "cross".into(), dictionary_id: "he".into(), rng_seed: 7, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(vec!["שלום".to_string()], false)));
        let rules = CrosswordRules { free_word_mode: false, reading_dir: ReadingDirection::RTL, ..Default::default() };
        let center = CrosswordRules::center_cell(&st.board.geom);
        let base = st.board.geom.from_cell_id(center).unwrap();
        let placements = vec![
            (st.board.geom.to_cell_id(Coord2D { x: base.x - 2, y: base.y }).unwrap(), Tile { kind_id: "MEMF".into(), mark: None }),
            (st.board.geom.to_cell_id(Coord2D { x: base.x - 1, y: base.y }).unwrap(), Tile { kind_id: "VAV".into(), mark: None }),
            (center, Tile { kind_id: "LAMED".into(), mark: None }),
            (st.board.geom.to_cell_id(Coord2D { x: base.x + 1, y: base.y }).unwrap(), Tile { kind_id: "SHIN".into(), mark: None }),
        ];
        let mv = MoveDraft { placements };
        let validated = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &validated);
        assert_eq!(sc.main_word, "שלום");
        assert!(sc.main_score > 0);
    }

    #[test]
    fn rtl_arabic_word_validates() {
        let tileset = Tileset { tile_kinds: vec![
            TileKind { id: "SEEN".into(), symbol: "س".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "LAM".into(), symbol: "ل".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "ALEF".into(), symbol: "ا".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "MEEM".into(), symbol: "م".into(), score: 1, is_blank: false, aliases: vec![] },
        ]};
        let mut counts = HashMap::new(); counts.insert("SEEN".to_string(), 4); counts.insert("LAM".to_string(), 4); counts.insert("ALEF".to_string(), 4); counts.insert("MEEM".to_string(), 4);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 7, height: 7 }, ruleset_id: "cross".into(), dictionary_id: "ar".into(), rng_seed: 11, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(FstDictionary::from_words(vec!["سلام".to_string()], false)));
        let rules = CrosswordRules { free_word_mode: false, reading_dir: ReadingDirection::RTL, ..Default::default() };
        let center = CrosswordRules::center_cell(&st.board.geom);
        let base = st.board.geom.from_cell_id(center).unwrap();
        let placements = vec![
            (st.board.geom.to_cell_id(Coord2D { x: base.x - 2, y: base.y }).unwrap(), Tile { kind_id: "MEEM".into(), mark: None }),
            (st.board.geom.to_cell_id(Coord2D { x: base.x - 1, y: base.y }).unwrap(), Tile { kind_id: "ALEF".into(), mark: None }),
            (center, Tile { kind_id: "LAM".into(), mark: None }),
            (st.board.geom.to_cell_id(Coord2D { x: base.x + 1, y: base.y }).unwrap(), Tile { kind_id: "SEEN".into(), mark: None }),
        ];
        let mv = MoveDraft { placements };
        let validated = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &validated);
        assert_eq!(sc.main_word, "سلام");
        assert!(sc.main_score > 0);
    }
}

#[cfg(test)]
mod more_rule_tests {
    use super::*;
    use crate::{dict::{FstDictionary, SetDictionary}, GameConfig, RectBoardLayout};
    use std::collections::HashMap;

    #[test]
    fn first_move_must_cover_center_and_contiguous() {
        let tileset = Tileset { tile_kinds: vec![TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] }] };
        let mut counts = HashMap::new(); counts.insert("A".to_string(), 10);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "crossword_classic".into(), dictionary_id: "en".into(), rng_seed: 1, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let rules = CrosswordRules::default();
        let off = st.board.geom.to_cell_id(Coord2D { x: 0, y: 0 }).unwrap();
        let mv = MoveDraft { placements: vec![(off, Tile { kind_id: "A".into(), mark: None })] };
        assert!(rules.validate(&st, &mv).is_err());
        let cen = CrosswordRules::center_cell(&st.board.geom);
        let mv = MoveDraft { placements: vec![(cen, Tile { kind_id: "A".into(), mark: None })] };
        let v = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &v);
        assert!(sc.total >= 1);
        rules.commit(&mut st, v, &sc).unwrap();
    }


    #[test]
    fn emoji_grapheme_word_scoring() {
        let emoji = "👩‍🚀";
        let tileset = Tileset { tile_kinds: vec![
            TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "EM".into(), symbol: emoji.into(), score: 5, is_blank: false, aliases: vec![] },
        ]};
        let mut counts = HashMap::new(); counts.insert("A".to_string(), 10); counts.insert("EM".to_string(), 10);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 11, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let word = format!("A{}", emoji);
        st.dictionary = Some(Box::new(FstDictionary::from_words(vec![word.clone()], false)));
        let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
        let c = CrosswordRules::center_cell(&st.board.geom);
        let cc = st.board.geom.from_cell_id(c).unwrap();
        let right = st.board.geom.to_cell_id(Coord2D { x: cc.x + 1, y: cc.y }).unwrap();
        let mv = MoveDraft { placements: vec![(c, Tile { kind_id: "A".into(), mark: None }), (right, Tile { kind_id: "EM".into(), mark: None })] };
        let v = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &v);
        assert_eq!(sc.main_word, word);
        assert_eq!(sc.total, 1 + 5);
    }

    #[test]
    fn emoji_skin_tone_grapheme_mixed_with_letter() {
        let thumbs = "👍🏽";
        let tileset = Tileset { tile_kinds: vec![
            TileKind { id: "EM2".into(), symbol: thumbs.into(), score: 4, is_blank: false, aliases: vec![] },
            TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
        ]};
        let mut counts = HashMap::new(); counts.insert("EM2".to_string(), 10); counts.insert("A".to_string(), 10);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 13, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        let word = format!("{}A", thumbs);
        st.dictionary = Some(Box::new(FstDictionary::from_words(vec![word.clone()], false)));
        let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
        let c = CrosswordRules::center_cell(&st.board.geom);
        let cc = st.board.geom.from_cell_id(c).unwrap();
        let right = st.board.geom.to_cell_id(Coord2D { x: cc.x + 1, y: cc.y }).unwrap();
        let mv = MoveDraft { placements: vec![(c, Tile { kind_id: "EM2".into(), mark: None }), (right, Tile { kind_id: "A".into(), mark: None })] };
        let v = rules.validate(&st, &mv).unwrap();
        let sc = rules.score(&st, &v);
        assert_eq!(sc.main_word, word);
        assert_eq!(sc.total, 4 + 1);
    }


    #[test]
    fn dictionary_integration_in_rules() {
        let tileset = Tileset { tile_kinds: vec![
            TileKind { id: "A".into(), symbol: "A".into(), score: 1, is_blank: false, aliases: vec![] },
            TileKind { id: "B".into(), symbol: "B".into(), score: 3, is_blank: false, aliases: vec![] },
        ]};
        let mut counts = HashMap::new(); counts.insert("A".to_string(), 10); counts.insert("B".to_string(), 10);
        let cfg = GameConfig { tileset, rack_size: 7, board_layout: RectBoardLayout { width: 5, height: 5 }, ruleset_id: "cross".into(), dictionary_id: "en".into(), rng_seed: 5, tile_counts: counts };
        let mut st = GameState::new(&cfg, 2).unwrap();
        st.dictionary = Some(Box::new(SetDictionary::from_words(vec!["AB".to_string()], true)));
        let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
        let c = CrosswordRules::center_cell(&st.board.geom);
        let mv1 = MoveDraft { placements: vec![(c, Tile { kind_id: "A".into(), mark: None })] };
        let v1 = rules.validate(&st, &mv1).unwrap();
        let sc1 = rules.score(&st, &v1);
        assert_eq!(sc1.main_score, -1);
        let right = st.board.geom.to_cell_id(Coord2D { x: st.board.geom.from_cell_id(c).unwrap().x + 1, y: st.board.geom.from_cell_id(c).unwrap().y }).unwrap();
        let mv2 = MoveDraft { placements: vec![(c, Tile { kind_id: "A".into(), mark: None }), (right, Tile { kind_id: "B".into(), mark: None })] };
        let v2 = rules.validate(&st, &mv2).unwrap();
        let sc2 = rules.score(&st, &v2);
        assert!(sc2.total > 0);
    }
}
