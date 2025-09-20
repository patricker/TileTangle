import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createRequire } from 'node:module';
import assert from 'node:assert/strict';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const pkgDir = path.join(__dirname, 'pkg');
// For Node target, the wasm-pack module is CommonJS; use require for compatibility
const require = createRequire(import.meta.url);
const mod = require(path.join(pkgDir, 'tiletangle_wasm.js'));

const cfg = {
  tileset: { tile_kinds: [{ id: 'A', symbol: 'A', score: 1 }] },
  rack_size: 7,
  board_layout: { width: 3, height: 2 },
  ruleset_id: 'cross',
  dictionary_id: 'en',
  rng_seed: 1,
  tile_counts: { A: 0 },
  free_word_mode: true,
};

const game = mod.new_game(JSON.stringify(cfg), 2);
const board = JSON.parse(mod.get_board(game));
assert.equal(board.width, 3);
assert.equal(board.height, 2);
console.log('WASM smoke test passed');
