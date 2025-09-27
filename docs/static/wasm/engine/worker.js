// Simple WASM worker that initializes the engine once and responds to messages
let mod = null;
let game = null;

async function ensureInit() {
  if (!mod) {
    // Use a relative path so this works under any baseUrl
    mod = await import('./pkg/tiletangle_wasm.js');
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
    } else if (action === 'set_rack') {
      if (!game) throw new Error('no game');
      const tiles = Array.isArray(payload?.tiles) ? payload.tiles : [];
      mod.set_rack(game, JSON.stringify(tiles));
      self.postMessage({ id, ok: true });
    } else if (action === 'set_reading_direction') {
      if (!game) throw new Error('no game');
      mod.set_reading_direction(game, !!payload.rtl);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_stacking') {
      if (!game) throw new Error('no game');
      const sum = (payload.scoring === 'sum');
      mod.set_stacking(game, !!payload.enabled, payload.max_height ?? 7, !!payload.forbid_same, sum);
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
    } else if (action === 'snapshot_json') {
      if (!game) throw new Error('no game');
      const snap = mod.snapshot_state_json(game);
      self.postMessage({ id, ok: true, snapshot: snap });
    } else if (action === 'snapshot_cbor') {
      if (!game) throw new Error('no game');
      const snap = mod.snapshot_state_cbor(game);
      self.postMessage({ id, ok: true, snapshot: snap });
    } else if (action === 'load_snapshot_json') {
      if (!game) throw new Error('no game');
      mod.load_state_json(game, payload?.json ?? '');
      self.postMessage({ id, ok: true });
    } else if (action === 'load_snapshot_cbor') {
      if (!game) throw new Error('no game');
      mod.load_state_cbor(game, payload?.bytes ?? new Uint8Array());
      self.postMessage({ id, ok: true });
    } else if (action === 'event_log') {
      if (!game) throw new Error('no game');
      const log = mod.get_event_log(game);
      self.postMessage({ id, ok: true, log });
    } else if (action === 'set_bonuses') {
      if (!game) throw new Error('no game');
      mod.set_bonuses(game, JSON.stringify(payload || []));
      self.postMessage({ id, ok: true });
    } else if (action === 'generate_moves') {
      if (!game) throw new Error('no game');
      const moves = mod.generate_moves(game, payload?.max_len ?? 7, payload?.limit ?? 50);
      self.postMessage({ id, ok: true, moves });
    } else if (action === 'best_move') {
      if (!game) throw new Error('no game');
      const seed = payload?.seed ?? undefined;
      const optsRaw = {
        node_limit: payload?.node_limit,
        time_limit_ms: payload?.time_limit_ms,
        difficulty: payload?.difficulty,
        noise_range: payload?.noise_range,
        candidate_limit: payload?.candidate_limit,
        reply_limit: payload?.reply_limit,
        parallel_eval: payload?.parallel_eval,
        opponent: payload?.opponent,
      };
      const opts = Object.fromEntries(
        Object.entries(optsRaw).filter(([, value]) => value !== undefined && value !== null),
      );
      const res = mod.best_move_greedy(
        game,
        payload?.max_len ?? undefined,
        payload?.depth ?? undefined,
        seed,
        Object.keys(opts).length ? opts : undefined,
      );
      self.postMessage({ id, ok: true, best: res });
    } else if (action === 'evaluate_candidate') {
      if (!game) throw new Error('no game');
      const optsRaw = { difficulty: payload?.difficulty };
      const opts = Object.fromEntries(Object.entries(optsRaw).filter(([,v]) => v != null));
      const res = mod.evaluate_candidate(game, JSON.stringify(payload?.placements || []), opts);
      self.postMessage({ id, ok: true, eval: res });
    } else if (action === 'pass_turn') {
      if (!game) throw new Error('no game');
      mod.pass_turn(game);
      self.postMessage({ id, ok: true });
    } else if (action === 'exchange_tiles') {
      if (!game) throw new Error('no game');
      mod.exchange_tiles(game, JSON.stringify(payload?.kinds ?? []));
      self.postMessage({ id, ok: true });
    } else if (action === 'undo') {
      if (!game) throw new Error('no game');
      mod.undo(game);
      self.postMessage({ id, ok: true });
    } else if (action === 'redo') {
      if (!game) throw new Error('no game');
      mod.redo(game);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_dictionary_from_fst_bytes') {
      if (!game) throw new Error('no game');
      mod.set_dictionary_from_fst_bytes(game, payload.bytes, !!payload.case_fold);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_dictionary_from_text') {
      if (!game) throw new Error('no game');
      mod.set_dictionary_from_text(game, payload.text, !!payload.case_fold);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_dictionary_from_gaddag_bytes') {
      if (!game) throw new Error('no game');
      mod.set_dictionary_from_gaddag_bytes(game, payload.bytes, !!payload.case_fold);
      self.postMessage({ id, ok: true });
    } else if (action === 'set_dictionary_engine') {
      if (!game) throw new Error('no game');
      mod.set_dictionary_from_text_engine(game, payload.text, payload.engine, !!payload.case_fold);
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
