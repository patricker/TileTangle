---
sidebar_position: 7
---

# Web (WASM) API

Build the WASM package:

```
make wasm
```

Load in the browser (example):

```html
<script type="module">
  import init, { new_game, play_move, get_board } from '/static/wasm/engine/pkg/tiletangle_wasm.js';
  await init();
  const config = {
    tileset: { tile_kinds: [{ id: 'A', symbol: 'A', score: 1 }] },
    rack_size: 7,
    board_layout: { width: 7, height: 7 },
    ruleset_id: 'crossword_classic',
    dictionary_id: 'en_demo',
    rng_seed: 42,
    tile_counts: { A: 10 },
    free_word_mode: true
  };
  const game = new_game(JSON.stringify(config), 2);
  console.log('board', get_board(game));
  const placements = [{ x: 3, y: 3, kind_id: 'A' }];
  const result = play_move(game, JSON.stringify(placements));
  console.log('score', JSON.parse(result));
  console.log('board after', get_board(game));
 </script>
```

Artifacts are copied to `docs/static/wasm/engine/pkg` by `make wasm`.
