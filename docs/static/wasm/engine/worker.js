// Simple WASM worker that initializes the engine once and responds to messages
let mod = null;
let game = null;

async function ensureInit() {
  if (!mod) {
    mod = await import('/wasm/engine/pkg/tiletangle_wasm.js');
    await mod.default();
  }
}

self.onmessage = async (e) => {
  const { id, action, payload } = e.data || {};
  try {
    await ensureInit();
    if (action === 'new_game') {
      game = mod.new_game(JSON.stringify(payload.config), payload.players ?? 2);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_free_word_mode') {
      if (!game) throw new Error('no game');
      mod.set_free_word_mode(game, !!payload.on);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_reading_direction') {
      if (!game) throw new Error('no game');
      mod.set_reading_direction(game, !!payload.rtl);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_stacking') {
      if (!game) throw new Error('no game');
      mod.set_stacking(game, !!payload.enabled, payload.max_height ?? 7, !!payload.forbid_same, payload.scoring ?? 'top');
      self.postMessage({ id, ok: true });
    } else if (action === 'get_board') {
      const b = game ? mod.get_board(game) : null;
      self.postMessage({ id, ok: true, board: b });
    } else if (action === 'get_rack') {
      if (!game) throw new Error('no game');
      const r = mod.get_rack(game);
      self.postMessage({ id, ok: true, rack: r });
    } else if (action === 'get_scores') {
      if (!game) throw new Error('no game');
      const s = mod.get_scores(game);
      self.postMessage({ id, ok: true, scores: s });
    } else if (action === 'set_bonuses') {
      if (!game) throw new Error('no game');
      mod.set_bonuses(game, JSON.stringify(payload || []));
      self.postMessage({ id, ok: true });
    } else if (action === 'generate_moves') {
      if (!game) throw new Error('no game');
      const moves = mod.generate_moves(game, payload?.max_len ?? 7, payload?.limit ?? 50);
      self.postMessage({ id, ok: true, moves });
    } else if (action === 'pass_turn') {
      if (!game) throw new Error('no game');
      mod.pass_turn(game);
      self.postMessage({ id, ok: true });
    } else if (action === 'exchange_tiles') {
      if (!game) throw new Error('no game');
      mod.exchange_tiles(game, JSON.stringify(payload?.kinds ?? []));
      self.postMessage({ id, ok: true });
    } else if (action === 'set_dictionary_from_fst_bytes') {
      if (!game) throw new Error('no game');
      mod.set_dictionary_from_fst_bytes(game, payload.bytes, !!payload.case_fold);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_dictionary_from_text') {
      if (!game) throw new Error('no game');
      mod.set_dictionary_from_text(game, payload.text, !!payload.case_fold);
      self.postMessage({ id, ok: true });
    } else if (action === 'play_move') {
      if (!game) throw new Error('no game');
      const res = mod.play_move(game, JSON.stringify(payload.placements));
      self.postMessage({ id, ok: true, result: res });
    } else {
      throw new Error('unknown action');
    }
  } catch (err) {
    self.postMessage({ id, ok: false, error: String(err) });
  }
};
