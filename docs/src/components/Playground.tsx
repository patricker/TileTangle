import React, {useEffect, useMemo, useState} from 'react';

type BoardJson = { width: number; height: number; rows: string[][] };

export default function Playground(): JSX.Element {
  const [ready, setReady] = useState(false);
  const [game, setGame] = useState<any>(null);
  const [board, setBoard] = useState<BoardJson | null>(null);

  useEffect(() => {
    (async () => {
      // Dynamic import from Docusaurus static dir
      const mod = await import('/wasm/engine/pkg/tiletangle_wasm.js');
      await mod.default(); // init
      const cfg = {
        tileset: { tile_kinds: [{ id: 'A', symbol: 'A', score: 1 }] },
        rack_size: 7,
        board_layout: { width: 7, height: 7 },
        ruleset_id: 'cross', dictionary_id: 'en', rng_seed: 1,
        tile_counts: { A: 30 }, free_word_mode: true,
      };
      const g = mod.new_game(JSON.stringify(cfg), 2);
      setGame({mod, g});
      setBoard(JSON.parse(mod.get_board(g)) as BoardJson);
      setReady(true);
    })();
  }, []);

  const placeCenter = async () => {
    if (!game) return;
    const placements = [{x: 3, y: 3, kind_id: 'A'}];
    try { game.mod.play_move(game.g, JSON.stringify(placements)); } catch (e) { console.error(e); }
    setBoard(JSON.parse(game.mod.get_board(game.g)) as BoardJson);
  };

  if (!ready || !board) return <div>Loading WASM…</div>;

  return (
    <div>
      <div style={{marginBottom: 12}}>
        <button onClick={placeCenter}>Place A at center</button>
      </div>
      <div style={{display: 'grid', gridTemplateColumns: `repeat(${board.width}, 28px)`, gap: 4}}>
        {Array.from({length: board.height}).map((_, y) => (
          Array.from({length: board.width}).map((__, x) => (
            <div key={`${x}-${y}`} style={{width: 28, height: 28, border: '1px solid #ccc', display:'flex', alignItems:'center', justifyContent:'center'}}>
              {(board.rows[y][x] || '').slice(0,1)}
            </div>
          ))
        ))}
      </div>
    </div>
  );
}

