use std::collections::{HashMap as Map, HashSet as Set};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    geometry::BoardGeometry,
    geometry::{CellId, Coord2D, RectGridGeometry},
    inventory::{Tileset},
    rules::{Rules, crossword::{CrosswordRules, ValidatedMove}},
    dict::{GaddagDictionary, GaddagCursor},
    game::GameState,
    Tile, TileKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateMove {
    pub placements: Vec<(CellId, Tile)>,
    pub word: String,
    pub score: i32,
}

/// Generate naive moves at anchors. Rack is a multiset of `kind_id` strings.
pub fn generate_moves(
    state: &GameState,
    rules: &impl Rules,
    rack: &[String],
    max_len: usize,
) -> Vec<CandidateMove> {
    // find anchors: empty cells adjacent to any existing tile; if board empty, use center
    let mut anchors: Vec<CellId> = Vec::new();
    let any_on_board = state.board.cells.iter().any(|c| !c.stack.is_empty());
    if !any_on_board {
        anchors.push(CrosswordRules::center_cell(&state.board.geom));
    } else {
        for (idx, cell) in state.board.cells.iter().enumerate() {
            if !cell.stack.is_empty() { continue; }
            let id = CellId(idx as u32);
            if CrosswordRules::adjacent_to_existing(&state.board, &[id]) { anchors.push(id); }
        }
    }

    // if geometry has tags/overlay, use the tag-aware generator
    if state.board.geom.has_graph() { return generate_moves_graph_basic(state, rules, rack, max_len, &anchors); }

    fn get_symbol<'a>(tileset: &'a Tileset, kind_id: &str) -> Option<(&'a TileKind, &'a str)> {
        for tk in &tileset.tile_kinds { if tk.id == kind_id { return Some((tk, tk.symbol.as_str())); } }
        None
    }
    fn find_kind_for_symbol<'a>(tileset: &'a Tileset, sym: &str) -> Option<&'a TileKind> {
        tileset.tile_kinds.iter().find(|tk| !tk.is_blank && tk.symbol == sym)
    }

    // build helper to read cell including overlay
    #[derive(Default, Clone)]
    struct Overlay(Map<CellId, Tile>);
    impl Overlay {
        fn get<'a>(&'a self, id: CellId, state: &'a GameState) -> Option<&'a Tile> {
            if let Some(t) = self.0.get(&id) { return Some(t); }
            let cell = &state.board.cells[id.0 as usize];
            cell.stack.last()
        }
    }

    fn perp_word(state: &GameState, ov: &Overlay, at: CellId, dir: (i32, i32)) -> String {
        let c0 = state.board.geom.from_cell_id(at).unwrap();
        let (dx, dy) = dir;
        let mut c = c0;
        loop {
            let prev = Coord2D { x: c.x - dx, y: c.y - dy };
            if let Some(id) = state.board.geom.to_cell_id(prev) && ov.get(id, state).is_some() { c = prev; continue; }
            break;
        }
        let mut s = String::new();
        loop {
            if let Some(id) = state.board.geom.to_cell_id(c) && let Some(tile) = ov.get(id, state) {
                let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, tile);
                s.push_str(&sym);
                c = Coord2D { x: c.x + dx, y: c.y + dy };
                continue;
            }
            break;
        }
        s
    }

    // Precompute cross-check sets (only when dictionary enforced)
    fn gather_line(state: &GameState, start: Coord2D, step: (i32, i32)) -> (String, usize) {
        let mut s = String::new();
        let mut tiles = 0usize;
        let mut c = start;
        loop {
            if let Some(id) = state.board.geom.to_cell_id(c) {
                let cell = &state.board.cells[id.0 as usize];
                if let Some(t) = cell.stack.last() {
                    let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, t);
                    s.push_str(&sym);
                    tiles += 1;
                    c = Coord2D { x: c.x + step.0, y: c.y + step.1 };
                    continue;
                }
            }
            break;
        }
        (s, tiles)
    }
    fn cross_checks(state: &GameState, dir: (i32, i32)) -> Map<CellId, Set<String>> {
        let mut out: Map<CellId, Set<String>> = Map::new();
        for (idx, cell) in state.board.cells.iter().enumerate() {
            if !cell.stack.is_empty() { continue; }
            let id = CellId(idx as u32);
            let c = state.board.geom.from_cell_id(id).unwrap();
            let above = Coord2D { x: c.x - dir.0, y: c.y - dir.1 };
            let below = Coord2D { x: c.x + dir.0, y: c.y + dir.1 };
            let (top, top_tiles) = gather_line(state, above, (-dir.0, -dir.1));
            let (bot, bot_tiles) = gather_line(state, below, (dir.0, dir.1));
            let base_tiles = top_tiles + bot_tiles;
            if base_tiles == 0 { continue; }
            let mut set: Set<String> = Set::new();
            if let Some(dict) = &state.dictionary {
                for tk in &state.tileset.tile_kinds {
                    if tk.is_blank { continue; }
                    let sym = &tk.symbol;
                    let w = format!("{}{}{}", top, sym, bot);
                    if base_tiles + 1 > 1 && dict.contains(&w) { set.insert(sym.clone()); }
                }
            } else {
                for tk in &state.tileset.tile_kinds { if !tk.is_blank { set.insert(tk.symbol.clone()); } }
            }
            out.insert(id, set);
        }
        out
    }
    // vertical cross checks for horizontal plays, and horizontal for vertical plays
    let xchecks_vert = cross_checks(state, (0, 1));
    let xchecks_horz = cross_checks(state, (1, 0));

    #[allow(
        clippy::too_many_arguments,
        clippy::collapsible_if,
        clippy::manual_retain
    )]
    fn dfs_right(
        state: &GameState,
        rules: &impl Rules,
        anchor: CellId,
        rack: &mut Map<String, usize>,
        built: String,
        pos: Coord2D,
        used: &mut Vec<(CellId, Tile)>,
        out: &mut Vec<CandidateMove>,
        max_len: usize,
        dir: (i32, i32),
        pdir: (i32, i32),
        mut gaddag: Option<(&GaddagDictionary, usize)>,
        xchecks: &Map<CellId, Set<String>>,
        blank_kinds: &[String],
    ) {
        let len_tokens = if let Some((gd, _)) = &gaddag { gd.tokenizer().segment(&built).len() } else { built.graphemes(true).count() };
        if len_tokens >= max_len { return; }
        if let Some(id) = state.board.geom.to_cell_id(pos) {
            let cell = &state.board.cells[id.0 as usize];
            if let Some(t) = cell.stack.last() {
                let mut nb = built.clone();
                let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, t);
                nb.push_str(&sym);
                if let Some((gd, node)) = gaddag {
                    if let Some(n2) = gd.step_symbol(node, &sym) { gaddag = Some((gd, n2)); } else { return; }
                } else if let Some(dict) = &state.dictionary { if !dict.has_prefix(&nb) { return; } }
                let next = Coord2D { x: pos.x + dir.0, y: pos.y + dir.1 };
                dfs_right(state, rules, anchor, rack, nb, next, used, out, max_len, dir, pdir, gaddag, xchecks, blank_kinds);
                return;
            }
        } else { return; }

        let id = state.board.geom.to_cell_id(pos).unwrap();
        if state.board.cells[id.0 as usize].stack.is_empty() {
            let mut cand_syms: Set<String> = Set::new();
            if let Some(set) = xchecks.get(&id) { for s in set { cand_syms.insert(s.clone()); } }
            else if let Some(dict) = &state.dictionary {
                for tk in &state.tileset.tile_kinds {
                    if tk.is_blank { continue; }
                    if dict.has_prefix(&tk.symbol) {
                        cand_syms.insert(tk.symbol.clone());
                    }
                }
            } else {
                for tk in &state.tileset.tile_kinds {
                    if !tk.is_blank {
                        cand_syms.insert(tk.symbol.clone());
                    }
                }
            }
            if let Some((gd, node)) = gaddag { cand_syms = cand_syms.into_iter().filter(|s| gd.step_symbol(node, s).is_some()).collect(); }
            let mut blank_syms = cand_syms.clone();
            if let Some(dict) = &state.dictionary { for ch in 'A'..='Z' { let s = ch.to_string(); if dict.has_prefix(&s) { blank_syms.insert(s); } } }
            for sym in blank_syms.into_iter() {
                let normal_kind = find_kind_for_symbol(&state.tileset, &sym).map(|k| k.id.clone());
                let normal_avail = normal_kind.as_ref().and_then(|kid| rack.get(kid)).copied().unwrap_or(0) > 0;
                if normal_avail { continue; }
                let mut chosen_blank: Option<String> = None;
                for bk in blank_kinds { if rack.get(bk).copied().unwrap_or(0) > 0 { chosen_blank = Some(bk.clone()); break; } }
                if let Some(bid) = chosen_blank {
                    let mut ov = Overlay::default();
                    for (cid, tile) in used.iter() { ov.0.insert(*cid, tile.clone()); }
                    ov.0.insert(id, Tile { kind_id: bid.clone(), mark: Some(sym.clone()) });
                    let vword = perp_word(state, &ov, id, pdir);
                    let vtiles = {
                        let c0 = id; let mut count = 0usize; let mut c = state.board.geom.from_cell_id(c0).unwrap(); let (dx, dy) = pdir;
                        loop { let prev = Coord2D { x: c.x - dx, y: c.y - dy }; if let Some(pid) = state.board.geom.to_cell_id(prev) { if ov.get(pid, state).is_some() { c = prev; continue; } } break; }
                        loop { if let Some(pid) = state.board.geom.to_cell_id(c) { if ov.get(pid, state).is_some() { count += 1; c = Coord2D { x: c.x + dx, y: c.y + dy }; continue; } } break; }
                        count
                    };
                    if vtiles > 1 { if let Some(dict) = &state.dictionary { if !dict.contains(&vword) { continue; } } }
                    let mut nb = built.clone(); nb.push_str(&sym);
                    let mut next_gaddag = gaddag;
                    if let Some((gd, node)) = next_gaddag { if let Some(n2) = gd.step_symbol(node, &sym) { next_gaddag = Some((gd, n2)); } else { continue; } }
                    *rack.get_mut(&bid).unwrap() -= 1;
                    used.push((id, Tile { kind_id: bid.clone(), mark: Some(sym.clone()) }));
                    if used.iter().any(|(cid, _)| *cid == anchor) || state.board.cells[anchor.0 as usize].stack.last().is_some() {
                        let v = ValidatedMove { placements: used.clone(), line_is_row: dir.1 == 0 };
                        let sc = rules.score(state, &v);
                        if sc.total >= 0 { out.push(CandidateMove { placements: used.clone(), word: sc.main_word.clone(), score: sc.total }); }
                    }
                    let next = Coord2D { x: pos.x + dir.0, y: pos.y + dir.1 };
                    dfs_right(state, rules, anchor, rack, nb, next, used, out, max_len, dir, pdir, next_gaddag, xchecks, blank_kinds);
                    used.pop(); *rack.get_mut(&bid).unwrap() += 1;
                }
            }

            for (kind_id, cnt) in rack.clone() {
                if cnt == 0 { continue; }
                let Some((_tk, sym)) = get_symbol(&state.tileset, &kind_id) else { continue; };
                let mut ov = Overlay::default();
                for (cid, tile) in used.iter() { ov.0.insert(*cid, tile.clone()); }
                ov.0.insert(id, Tile { kind_id: kind_id.clone(), mark: None });
                let vword = perp_word(state, &ov, id, pdir);
                let vtiles = {
                    let mut count = 0usize; let mut c = state.board.geom.from_cell_id(id).unwrap(); let (dx, dy) = pdir;
                    loop { let prev = Coord2D { x: c.x - dx, y: c.y - dy }; if let Some(pid) = state.board.geom.to_cell_id(prev) { if ov.get(pid, state).is_some() { c = prev; continue; } } break; }
                    loop { if let Some(pid) = state.board.geom.to_cell_id(c) { if ov.get(pid, state).is_some() { count += 1; c = Coord2D { x: c.x + dx, y: c.y + dy }; continue; } } break; }
                    count
                };
                if vtiles > 1 { if let Some(dict) = &state.dictionary { if !dict.contains(&vword) { continue; } } }
                let mut nb = built.clone(); nb.push_str(sym);
                let mut next_gaddag = gaddag;
                if let Some((gd, node)) = next_gaddag { if let Some(n2) = gd.step_symbol(node, sym) { next_gaddag = Some((gd, n2)); } else { continue; } }
                *rack.get_mut(&kind_id).unwrap() -= 1;
                used.push((id, Tile { kind_id: kind_id.clone(), mark: None }));
                if used.iter().any(|(cid, _)| *cid == anchor) || state.board.cells[anchor.0 as usize].stack.last().is_some() {
                    let v = ValidatedMove { placements: used.clone(), line_is_row: dir.1 == 0 };
                    let sc = rules.score(state, &v);
                    if sc.total >= 0 { out.push(CandidateMove { placements: used.clone(), word: sc.main_word.clone(), score: sc.total }); }
                }
                let next = Coord2D { x: pos.x + dir.0, y: pos.y + dir.1 };
                dfs_right(state, rules, anchor, rack, nb, next, used, out, max_len, dir, pdir, next_gaddag, xchecks, blank_kinds);
                used.pop(); *rack.get_mut(&kind_id).unwrap() += 1;
            }
        }
    }

    let mut out: Vec<CandidateMove> = Vec::new();

    #[allow(
        clippy::too_many_arguments,
        clippy::collapsible_if,
        clippy::collapsible_else_if,
        clippy::manual_retain
    )]
    fn dfs_left_then_right(
        state: &GameState,
        rules: &impl Rules,
        anchor: CellId,
        rack: &mut Map<String, usize>,
        built: String,
        start: Coord2D,
        used: &mut Vec<(CellId, Tile)>,
        out: &mut Vec<CandidateMove>,
        max_len: usize,
        dir: (i32, i32),
        pdir: (i32, i32),
        gaddag_pre: Option<(&GaddagDictionary, usize)>,
        xchecks: &Map<CellId, Set<String>>,
        blank_kinds: &[String],
    ) {
        let mut post_sep = None;
        if let Some((gd, node)) = gaddag_pre { if let Some(n2) = gd.step_token(node, gd.sep_token()) { post_sep = Some((gd, n2)); } }
        dfs_right(state, rules, anchor, rack, built.clone(), start, used, out, max_len, dir, pdir, post_sep, xchecks, blank_kinds);
        let left = Coord2D { x: start.x - dir.0, y: start.y - dir.1 };
        if let Some(left_id) = state.board.geom.to_cell_id(left) {
            if !state.board.cells[left_id.0 as usize].stack.is_empty() { return; }
            let mut cand_syms: Set<String> = Set::new();
            if let Some(set) = xchecks.get(&left_id) { for s in set { cand_syms.insert(s.clone()); } }
            else if let Some(dict) = &state.dictionary {
                for tk in &state.tileset.tile_kinds {
                    if tk.is_blank { continue; }
                    if dict.has_prefix(&tk.symbol) {
                        cand_syms.insert(tk.symbol.clone());
                    }
                }
            } else {
                for tk in &state.tileset.tile_kinds {
                    if tk.is_blank { continue; }
                    cand_syms.insert(tk.symbol.clone());
                }
            }
            if let Some((gd, base)) = gaddag_pre { cand_syms = cand_syms.into_iter().filter(|s| gd.step_symbol(base, s).is_some()).collect(); }
            let mut blank_syms = cand_syms.clone();
            if let Some(dict) = &state.dictionary { for ch in 'A'..='Z' { let s = ch.to_string(); if dict.has_prefix(&s) { blank_syms.insert(s); } } }
            for sym in blank_syms.into_iter() {
                let normal_kind = find_kind_for_symbol(&state.tileset, &sym).map(|k| k.id.clone());
                let normal_avail = normal_kind.as_ref().and_then(|kid| rack.get(kid)).copied().unwrap_or(0) > 0;
                if normal_avail { continue; }
                let mut chosen_blank: Option<String> = None;
                for bk in blank_kinds { if rack.get(bk).copied().unwrap_or(0) > 0 { chosen_blank = Some(bk.clone()); break; } }
                if let Some(bid) = chosen_blank {
                    let mut nb = String::new(); nb.push_str(&sym); nb.push_str(&built);
                    let mut next_g_pre2 = gaddag_pre;
                    if let Some((gd, base2)) = next_g_pre2 { if let Some(n2) = gd.step_symbol(base2, &sym) { next_g_pre2 = Some((gd, n2)); } else { continue; } }
                    *rack.get_mut(&bid).unwrap() -= 1;
                    used.push((left_id, Tile { kind_id: bid.clone(), mark: Some(sym.clone()) }));
                    dfs_left_then_right(state, rules, anchor, rack, nb, left, used, out, max_len, dir, pdir, next_g_pre2, xchecks, blank_kinds);
                    used.pop(); *rack.get_mut(&bid).unwrap() += 1;
                }
            }
            for (kind_id, cnt) in rack.clone() {
                if cnt == 0 { continue; }
                let Some((_tk, sym)) = get_symbol(&state.tileset, &kind_id) else { continue; };
                let mut ov = Overlay::default();
                for (cid, tile) in used.iter() { ov.0.insert(*cid, tile.clone()); }
                ov.0.insert(left_id, Tile { kind_id: kind_id.clone(), mark: None });
                let vword = perp_word(state, &ov, left_id, pdir);
                let vtiles = {
                    let mut count = 0usize; let mut c = state.board.geom.from_cell_id(left_id).unwrap(); let (dx, dy) = pdir;
                    loop { let prev = Coord2D { x: c.x - dx, y: c.y - dy }; if let Some(pid) = state.board.geom.to_cell_id(prev) { if ov.get(pid, state).is_some() { c = prev; continue; } } break; }
                    loop { if let Some(pid) = state.board.geom.to_cell_id(c) { if ov.get(pid, state).is_some() { count += 1; c = Coord2D { x: c.x + dx, y: c.y + dy }; continue; } } break; }
                    count
                };
                if vtiles > 1 { if let Some(dict) = &state.dictionary { if !dict.contains(&vword) { continue; } } }
                let mut nb = String::new(); nb.push_str(sym); nb.push_str(&built);
                let mut next_g_pre = gaddag_pre;
                if let Some((gd, base)) = next_g_pre { if let Some(n2) = gd.step_symbol(base, sym) { next_g_pre = Some((gd, n2)); } else { continue; } }
                *rack.get_mut(&kind_id).unwrap() -= 1;
                used.push((left_id, Tile { kind_id: kind_id.clone(), mark: None }));
                dfs_left_then_right(state, rules, anchor, rack, nb, left, used, out, max_len, dir, pdir, next_g_pre, xchecks, blank_kinds);
                used.pop(); *rack.get_mut(&kind_id).unwrap() += 1;
            }
        }
    }

    let blank_kinds: Vec<String> = state.tileset.tile_kinds.iter().filter(|tk| tk.is_blank).map(|tk| tk.id.clone()).collect();
    // Seed optional GADDAG pruning from dictionary if present
    let mut gaddag_pre_seed: Option<&GaddagDictionary> = None;
    if let Some(dict) = &state.dictionary && let Some(gd) = dict.as_any().downcast_ref::<GaddagDictionary>() {
        gaddag_pre_seed = Some(gd);
    }

    for &a in &anchors {
        if !state.board.cells[a.0 as usize].stack.is_empty() { continue; }
        let start = state.board.geom.from_cell_id(a).unwrap();
        let mut rack_counts: Map<String, usize> = Map::new();
        for k in rack { *rack_counts.entry(k.clone()).or_default() += 1; }
        let h_prefix = context_prefix(state, start, (1, 0));
        let mut pre: Option<(&GaddagDictionary, usize)> = None;
        if let Some(gd) = gaddag_pre_seed && let Some(cur) = GaddagCursor::new(gd, &h_prefix) { pre = Some((gd, cur.pre_node())); }
        dfs_left_then_right(state, rules, a, &mut rack_counts.clone(), h_prefix, start, &mut Vec::new(), &mut out, max_len, (1, 0), (0, 1), pre, &xchecks_vert, &blank_kinds);
        let v_prefix = context_prefix(state, start, (0, 1));
        let mut pre_v: Option<(&GaddagDictionary, usize)> = None;
        if let Some(gd) = gaddag_pre_seed && let Some(cur) = GaddagCursor::new(gd, &v_prefix) { pre_v = Some((gd, cur.pre_node())); }
        dfs_left_then_right(state, rules, a, &mut rack_counts.clone(), v_prefix, start, &mut Vec::new(), &mut out, max_len, (0, 1), (1, 0), pre_v, &xchecks_horz, &blank_kinds);
    }
    // de-duplicate identical placement sets (same cells and assigned symbols)
    let mut seen: Set<String> = Set::new();
    let mut dedup: Vec<CandidateMove> = Vec::new();
    for cm in out.into_iter() {
        let mut key_parts: Vec<String> = cm.placements.iter().map(|(cid, t)| format!("{}:{}:{}", cid.0, t.kind_id, t.mark.clone().unwrap_or_default())).collect();
        key_parts.sort();
        let key = format!("{}|{}", cm.word, key_parts.join(","));
        if seen.insert(key) { dedup.push(cm); }
    }
    dedup.sort_by_key(|cm| (-cm.score, cm.word.clone(), cm.placements.len()));
    dedup
}

fn generate_moves_graph_basic(
    state: &GameState,
    rules: &impl Rules,
    rack: &[String],
    max_len: usize,
    anchors: &[CellId],
) -> Vec<CandidateMove> {
    use std::collections::{HashMap, HashSet};

    fn build_line(geom: &RectGridGeometry, start: CellId, tag: &str) -> Vec<CellId> {
        use std::collections::{HashSet, VecDeque};
        let mut visited: HashSet<CellId> = HashSet::new();
        let mut deque: VecDeque<CellId> = VecDeque::new();
        visited.insert(start);
        deque.push_back(start);
        for pass in 0..2 {
            let mut current = start;
            loop {
                let mut next_opt = None;
                for (n, t) in geom.neighbors_with_tags(current) {
                    if t == tag && !visited.contains(&n) { next_opt = Some(n); break; }
                }
                if let Some(next_id) = next_opt {
                    if pass == 0 { deque.push_back(next_id); } else { deque.push_front(next_id); }
                    visited.insert(next_id);
                    current = next_id;
                } else { break; }
            }
        }
        deque.into_iter().collect()
    }

    fn alphabet_symbols(state: &GameState) -> HashSet<String> {
        let mut set = HashSet::new();
        for tk in &state.tileset.tile_kinds { if !tk.is_blank { set.insert(tk.symbol.clone()); } }
        set
    }

    fn collect_chain_symbols_from(state: &GameState, mut next: CellId, mut prev: CellId, tag: &str) -> Vec<String> {
        let mut out = Vec::new();
        loop {
            let cell = &state.board.cells[next.0 as usize];
            if let Some(tile) = cell.stack.last() {
                let (_score, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, tile);
                out.push(sym);
            } else { break; }
            let mut advanced = false;
            for (n, t) in state.board.geom.neighbors_with_tags(next) {
                if t == tag && n != prev { prev = next; next = n; advanced = true; break; }
            }
            if !advanced { break; }
        }
        out
    }

    fn allowed_for_cell_on_axis(state: &GameState, cell: CellId, ctag: &str, alphabet: &HashSet<String>) -> HashSet<String> {
        let mut sides: Vec<CellId> = Vec::with_capacity(2);
        for (n, t) in state.board.geom.neighbors_with_tags(cell) { if t == ctag { sides.push(n); } }
        let (left_syms, right_syms) = match (sides.first(), sides.get(1)) {
            (None, None) => return alphabet.clone(),
            (Some(&a), None) | (None, Some(&a)) => { let rs = collect_chain_symbols_from(state, a, cell, ctag); if rs.is_empty() { return alphabet.clone(); } (Vec::new(), rs) }
            (Some(&a), Some(&b)) => { let ls = collect_chain_symbols_from(state, a, cell, ctag); let rs = collect_chain_symbols_from(state, b, cell, ctag); if ls.is_empty() && rs.is_empty() { return alphabet.clone(); } (ls, rs) }
        };
        let Some(dict) = state.dictionary.as_ref() else { return alphabet.clone(); };
        let mut allowed: HashSet<String> = HashSet::new();
        let base_tiles = left_syms.len() + right_syms.len();
        for sym in alphabet {
            let mut a = String::new();
            for s in left_syms.iter().rev() { a.push_str(s); }
            a.push_str(sym);
            for s in &right_syms { a.push_str(s); }
            let mut ok = base_tiles + 1 > 1 && dict.contains(&a);
            if !ok {
                let mut b = String::new();
                for s in right_syms.iter().rev() { b.push_str(s); }
                b.push_str(sym);
                for s in &left_syms { b.push_str(s); }
                ok = base_tiles + 1 > 1 && dict.contains(&b);
            }
            if ok || base_tiles == 0 { allowed.insert(sym.clone()); }
        }
        allowed
    }

    fn cross_checks_for_tag(state: &GameState, tag: &str, alphabet: &HashSet<String>) -> HashMap<CellId, HashSet<String>> {
        let mut map: HashMap<CellId, HashSet<String>> = HashMap::new();
        for (idx, cell) in state.board.cells.iter().enumerate() {
            if !cell.stack.is_empty() { continue; }
            let cid = CellId(idx as u32);
            let mut perp: HashSet<&str> = HashSet::new();
            for (_n, t) in state.board.geom.neighbors_with_tags(cid) { if t != tag { perp.insert(t); } }
            let mut allowed = alphabet.clone();
            for ctag in perp { let on_axis = allowed_for_cell_on_axis(state, cid, ctag, alphabet); allowed = allowed.intersection(&on_axis).cloned().collect(); }
            map.insert(cid, allowed);
        }
        map
    }

    struct ExploreCtx<'a> {
        state: &'a GameState,
        rules: &'a dyn Rules,
        tile_symbols: &'a [(String, String)],
        blank_ids: &'a [String],
        cross: &'a HashMap<CellId, HashSet<String>>,
        seen: &'a mut Set<String>,
        out: &'a mut Vec<CandidateMove>,
    }

    fn explore_segment(segment: &[CellId], idx: usize, rack_counts: &mut Map<String, usize>, placements: &mut Vec<(CellId, Tile)>, ctx: &mut ExploreCtx<'_>) {
        if idx == segment.len() {
            if placements.is_empty() { return; }
            let draft = crate::game::MoveDraft { placements: placements.clone() };
            if let Ok(validated) = ctx.rules.validate(ctx.state, &draft) {
                let sc = ctx.rules.score(ctx.state, &validated);
                if sc.total >= 0 {
                    let mut key_parts: Vec<String> = placements.iter().map(|(cid, tile)| format!("{}:{}:{}", cid.0, tile.kind_id, tile.mark.clone().unwrap_or_default())).collect();
                    key_parts.sort();
                    let key = format!("{}|{}", key_parts.join(";"), sc.main_word);
                    if ctx.seen.insert(key) { ctx.out.push(CandidateMove { placements: placements.clone(), word: sc.main_word, score: sc.total }); }
                }
            }
            return;
        }

        let cid = segment[idx];
        let cell = &ctx.state.board.cells[cid.0 as usize];
        if let Some(_tile) = cell.stack.last() {
            explore_segment(segment, idx + 1, rack_counts, placements, ctx);
            return;
        }

        let allowed_syms = ctx.cross.get(&cid);
        if let Some(allowed) = allowed_syms {
            for (kind_id, symbol) in ctx.tile_symbols.iter() {
                if !allowed.contains(symbol) { continue; }
                let available = rack_counts.get(kind_id).copied().unwrap_or(0);
                if available == 0 { continue; }
                { let entry = rack_counts.get_mut(kind_id).unwrap(); *entry -= 1; }
                placements.push((cid, Tile { kind_id: kind_id.clone(), mark: None }));
                explore_segment(segment, idx + 1, rack_counts, placements, ctx);
                placements.pop();
                { let entry = rack_counts.get_mut(kind_id).unwrap(); *entry += 1; }
            }

            for blank_id in ctx.blank_ids {
                let available = rack_counts.get(blank_id).copied().unwrap_or(0);
                if available == 0 { continue; }
                { let entry = rack_counts.get_mut(blank_id).unwrap(); *entry -= 1; }
                for symbol in allowed {
                    placements.push((cid, Tile { kind_id: blank_id.clone(), mark: Some(symbol.clone()) }));
                    explore_segment(segment, idx + 1, rack_counts, placements, ctx);
                    placements.pop();
                }
                { let entry = rack_counts.get_mut(blank_id).unwrap(); *entry += 1; }
            }
        }
    }

    let tile_symbols: Vec<(String, String)> = state.tileset.tile_kinds.iter().filter(|tk| !tk.is_blank).map(|tk| (tk.id.clone(), tk.symbol.clone())).collect();
    let blank_ids: Vec<String> = state.tileset.tile_kinds.iter().filter(|tk| tk.is_blank).map(|tk| tk.id.clone()).collect();

    let rack_template: Map<String, usize> = {
        let mut map = Map::new();
        for k in rack { *map.entry(k.clone()).or_default() += 1; }
        map
    };

    let mut out = Vec::new();
    let mut seen: Set<String> = Set::new();
    let ctx = ExploreCtx { state, rules: rules as &dyn Rules, tile_symbols: &tile_symbols, blank_ids: &blank_ids, cross: &HashMap::new(), seen: &mut seen, out: &mut out };

    for &anchor in anchors {
        if !state.board.cells[anchor.0 as usize].stack.is_empty() { continue; }
        let mut tags: HashSet<String> = HashSet::new();
        for (_n, tag) in state.board.geom.neighbors_with_tags(anchor) { tags.insert(tag.to_string()); }
        if tags.is_empty() { continue; }

        for tag in tags {
            let alphabet = alphabet_symbols(state);
            let cross_checks = cross_checks_for_tag(state, &tag, &alphabet);
            let line = build_line(&state.board.geom, anchor, &tag);
            if line.is_empty() { continue; }
            let Some(anchor_idx) = line.iter().position(|id| *id == anchor) else { continue; };
            let line_len = line.len();
            for start in 0..=anchor_idx {
                for end in anchor_idx..line_len {
                    let seg_len = end - start + 1;
                    if seg_len == 0 || seg_len > max_len { continue; }
                    let segment = &line[start..=end];
                    let blanks_needed = segment.iter().filter(|cid| state.board.cells[cid.0 as usize].stack.is_empty()).count();
                    if blanks_needed == 0 { continue; }
                    if blanks_needed > rack.len() { continue; }

                    let mut rack_counts = rack_template.clone();
                    let mut placements: Vec<(CellId, Tile)> = Vec::new();
                    let mut tag_ctx = ExploreCtx { state: ctx.state, rules: ctx.rules, tile_symbols: ctx.tile_symbols, blank_ids: ctx.blank_ids, cross: &cross_checks, seen: ctx.seen, out: ctx.out };
                    explore_segment(segment, 0, &mut rack_counts, &mut placements, &mut tag_ctx);
                }
            }
        }
    }

    out
}

fn context_prefix(state: &GameState, pos: Coord2D, dir: (i32, i32)) -> String {
    let mut c = pos;
    loop {
        let prev = Coord2D { x: c.x - dir.0, y: c.y - dir.1 };
        if let Some(id) = state.board.geom.to_cell_id(prev) && state.board.cells[id.0 as usize].stack.last().is_some() { c = prev; continue; }
        break;
    }
    let mut s = String::new();
    loop {
        if c.x == pos.x && c.y == pos.y { break; }
        if let Some(id) = state.board.geom.to_cell_id(c) && let Some(t) = state.board.cells[id.0 as usize].stack.last() {
            let (_, sym) = CrosswordRules::tile_symbol_and_score(&state.tileset, t);
            s.push_str(&sym);
            c = Coord2D { x: c.x + dir.0, y: c.y + dir.1 };
            continue;
        }
        break;
    }
    s
}
