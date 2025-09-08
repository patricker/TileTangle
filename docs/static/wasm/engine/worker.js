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
    } else if (action === 'get_board') {
      const b = game ? mod.get_board(game) : null;
      self.postMessage({ id, ok: true, board: b });
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

