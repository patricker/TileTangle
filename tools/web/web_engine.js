// Simple WASM adapter for Godot Web export.
// Exposes window.TT with the engine API methods Playground.gd expects.

import init, * as wasm from './tiletangle_wasm.js';

let wasmReady = false;
let game = null;
let tilesetLookup = {};
let dictionaryLoaded = false;

function ensureReady() {
  if (!wasmReady) throw new Error('WASM not initialized');
}

function buildCellsFromSnapshot(snap) {
  const out = { width: snap.board[0]?.length || 0, height: snap.board.length, cells: [] };
  for (let y = 0; y < snap.board.length; y++) {
    const row = snap.board[y];
    for (let x = 0; x < row.length; x++) {
      const stack = row[x] || [];
      const top = stack.length > 0 ? stack[stack.length - 1] : null;
      out.cells.push({ x, y, top });
    }
  }
  return out;
}

function packPlayersAndTurn(snap) {
  const players = (snap.scores || []).map(s => ({ score: s|0 }));
  return { players, to_move: snap.to_move|0 };
}

export async function setupWasm() {
  if (!wasmReady) {
    await init();
    wasmReady = true;
  }
}

window.TT = {
  async init() {
    await setupWasm();
  },
  new_game(config_json, players) {
    try {
      ensureReady();
      const cfg = JSON.parse(config_json);
      tilesetLookup = {};
      if (cfg && cfg.tileset && Array.isArray(cfg.tileset.tile_kinds)) {
        for (const k of cfg.tileset.tile_kinds) {
          tilesetLookup[k.id] = { symbol: k.symbol ?? k.id, score: k.score ?? 0, is_blank: !!k.is_blank };
        }
      }
      game = wasm.new_game(config_json, players);
      // Fire-and-forget dictionary load so CPU/move-gen use real lexicon
      dictionaryLoaded = false;
      (async () => { try { await this.load_dictionary(); } catch (e) { console.warn('Dictionary load failed:', e); } })();
      return !!game;
    } catch (e) {
      console.error('TT.new_game error', e);
      return false;
    }
  },
  async load_dictionary() {
    ensureReady(); if (!game) return false;
    const base = '';
    const tries = [
      'dictionaries/TWL06.gaddag.cbor.gz',
      'dictionaries/TWL06.fst',
      'dictionaries/TWL06.txt',
    ];
    for (const path of tries) {
      try {
        const res = await fetch(base + path);
        if (!res.ok) continue;
        if (path.endsWith('.txt')) {
          const text = await res.text();
          await wasm.set_dictionary_from_text_engine(game, text, 'fst', true);
        } else {
          const bytes = new Uint8Array(await res.arrayBuffer());
          if (path.endsWith('.fst')) {
            await wasm.set_dictionary_from_fst_bytes(game, bytes, true);
          } else {
            await wasm.set_dictionary_from_gaddag_bytes(game, bytes, true);
          }
        }
        dictionaryLoaded = true;
        return true;
      } catch (_) { /* try next */ }
    }
    return false;
  },
  set_bonuses(bonuses_json) {
    try { ensureReady(); if (!game) return false; wasm.set_bonuses(game, bonuses_json); return true; } catch { return false; }
  },
  get_board_cells_json() {
    try { ensureReady(); if (!game) return '{}'; const snap = JSON.parse(wasm.snapshot_state_json(game)); return JSON.stringify(buildCellsFromSnapshot(snap)); } catch (e) { console.error(e); return '{}'; }
  },
  get_rack_json() {
    try { ensureReady(); if (!game) return '[]'; const arr = JSON.parse(wasm.get_rack(game)); const out = arr.map(e => { const k = tilesetLookup[e.kind_id] || { symbol: e.kind_id, score: 0 }; return { kind_id: e.kind_id, symbol: k.symbol, score: k.score }; }); return JSON.stringify(out); } catch (e) { console.error(e); return '[]'; }
  },
  get_scores_json() {
    try { ensureReady(); if (!game) return '{}'; const snap = JSON.parse(wasm.snapshot_state_json(game)); const packed = packPlayersAndTurn(snap); return JSON.stringify(packed); } catch (e) { console.error(e); return '{}'; }
  },
  get_event_log_json(limit) {
    try { ensureReady(); if (!game) return '[]'; const arr = JSON.parse(wasm.get_event_log(game)); const out = Array.isArray(arr) ? arr.slice(Math.max(0, arr.length - (limit|0))) : []; return JSON.stringify(out); } catch (e) { console.error(e); return '[]'; }
  },
  preview_move(placements_json) {
    try {
      ensureReady(); if (!game) return '';
      const snap = wasm.snapshot_state_json(game);
      const res = wasm.play_move(game, placements_json); // commit
      // extract score and build minimal preview payload
      const score = typeof res === 'string' ? JSON.parse(res) : res;
      const placements = JSON.parse(placements_json).map(p => [p.x|0, p.y|0]);
      // restore snapshot
      wasm.load_state_json(game, snap);
      const payload = { total: score.total ?? 0, main_word: score.main_word ?? '', main_cells: placements, cross_cells: [] };
      return JSON.stringify(payload);
    } catch (e) {
      // On error, restore if snapshot taken
      try { if (game) { /* best effort: game may be invalid */ } } catch {}
      return '';
    }
  },
  play_move(placements_json) {
    try { ensureReady(); if (!game) return ''; const res = wasm.play_move(game, placements_json); return typeof res === 'string' ? res : JSON.stringify(res); } catch (e) { return ''; }
  },
  best_move(difficulty, seed) {
    try { ensureReady(); if (!game) return ''; return wasm.best_move(game, String(difficulty||'medium'), seed ?? 0); } catch (e) { return ''; }
  },
  generate_moves(max_len, limit) {
    try { ensureReady(); if (!game) return '[]'; return wasm.generate_moves(game, max_len|0, limit|0); } catch (e) { return '[]'; }
  },
  dictionary_status() { return dictionaryLoaded ? 'ready' : 'missing'; },
};

// Auto-init
setupWasm().catch(console.error);
