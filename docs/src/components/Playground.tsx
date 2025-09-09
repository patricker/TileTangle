import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';

type BoardJson = { width: number; height: number; rows: string[][] };
type Placement = { x: number; y: number; kind_id: string };

export default function Playground(): JSX.Element {
  const [ready, setReady] = useState(false);
  const [useWorker, setUseWorker] = useState(false);
  const [useDict, setUseDict] = useState(true);
  const [game, setGame] = useState<any>(null);
  const [board, setBoard] = useState<BoardJson | null>(null);
  const [pending, setPending] = useState<Placement[]>([]);
  const workerRef = useRef<Worker | null>(null);

  const cfg = useMemo(() => ({
    tileset: { tile_kinds: [
      { id: 'A', symbol: 'A', score: 1 },
      { id: 'B', symbol: 'B', score: 3 },
    ] },
    rack_size: 7,
    board_layout: { width: 9, height: 9 },
    ruleset_id: 'cross', dictionary_id: 'en', rng_seed: 1,
    tile_counts: { A: 30, B: 12 }, free_word_mode: true,
  }), []);

  useEffect(() => {
    (async () => {
      if (useWorker) {
        const w = new Worker('/wasm/engine/worker.js', { type: 'module' });
        workerRef.current = w;
        function call(action: string, payload?: any): Promise<any> {
          return new Promise((resolve, reject) => {
            const id = Math.random().toString(36).slice(2);
            const onMsg = (e: MessageEvent) => {
              if ((e.data as any)?.id === id) {
                w.removeEventListener('message', onMsg);
                (e.data as any).ok ? resolve(e.data) : reject(new Error((e.data as any).error));
              }
            };
            w.addEventListener('message', onMsg);
            w.postMessage({ id, action, payload });
          });
        }
        const cfg2 = { ...cfg, free_word_mode: !useDict } as any;
        await call('new_game', { config: cfg2, players: 2 });
        const { board: b } = await call('get_board');
        setGame({ call });
        setBoard(JSON.parse(b as string) as BoardJson);
        setReady(true);
      } else {
        const mod = await import('/wasm/engine/pkg/tiletangle_wasm.js');
        await mod.default();
        const cfg2 = { ...cfg, free_word_mode: !useDict } as any;
        const g = mod.new_game(JSON.stringify(cfg2), 2);
        if (useDict) {
          try {
            const resp = await fetch('/dictionaries/TWL06.fst');
            if (resp.ok) {
              const buf = new Uint8Array(await resp.arrayBuffer());
              mod.set_dictionary_from_fst_bytes(g, buf, true);
            } else {
              const txtResp = await fetch('/dictionaries/TWL06.txt');
              const txt = await txtResp.text();
              mod.set_dictionary_from_text(g, txt, true);
            }
          } catch (e) { console.error('Failed to load dictionary', e); }
        }
        setGame({ mod, g });
        setBoard(JSON.parse(mod.get_board(g)) as BoardJson);
        setReady(true);
      }
    })();
    return () => {
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, [useWorker, useDict, cfg]);

  const rack = useMemo(() => ['A','A','A','B','B'], []);

  const commit = async () => {
    if (!game || pending.length === 0) return;
    try {
      if (useWorker) {
        await game.call('play_move', { placements: pending });
        const { board: b } = await game.call('get_board');
        setBoard(JSON.parse(b as string) as BoardJson);
      } else {
        game.mod.play_move(game.g, JSON.stringify(pending));
        setBoard(JSON.parse(game.mod.get_board(game.g)) as BoardJson);
      }
      setPending([]);
    } catch (e) { console.error(e); }
  };

  const onDropCell = (x: number, y: number, ev: React.DragEvent<HTMLDivElement>) => {
    ev.preventDefault();
    const kind_id = ev.dataTransfer.getData('text/plain');
    if (!kind_id) return;
    setPending(prev => {
      if (prev.some(p => p.x === x && p.y === y)) return prev;
      // Prevent placing over existing board tile
      if (board && (board.rows[y][x] || '').length > 0) return prev;
      return [...prev, { x, y, kind_id }];
    });
  };

  const onDragStartTile = (k: string, ev: React.DragEvent<HTMLDivElement>) => {
    ev.dataTransfer.setData('text/plain', k);
  };

  const cellDisplay = (x: number, y: number): string => {
    const p = pending.find(pp => pp.x === x && pp.y === y);
    if (p) return p.kind_id;
    return (board?.rows[y][x] || '');
  };

  if (!ready || !board) return <div>Loading WASM…</div>;

  return (
    <div>
      <div style={{display:'flex', alignItems:'center', gap:12, marginBottom: 12}}>
        <label><input type="checkbox" checked={useWorker} onChange={e => setUseWorker(e.target.checked)} /> Use Web Worker</label>
        <label><input type="checkbox" checked={useDict} onChange={e => setUseDict(e.target.checked)} /> Dictionary checks (TWL06)</label>
        <button onClick={commit} disabled={pending.length === 0}>Commit Move ({pending.length})</button>
        <button onClick={() => setPending([])} disabled={pending.length === 0}>Reset</button>
      </div>
      <div style={{display:'flex', gap: 16, alignItems:'flex-start'}}>
        <div style={{display: 'grid', gridTemplateColumns: `repeat(${board.width}, 28px)`, gap: 4}}>
          {Array.from({length: board.height}).map((_, y) => (
            Array.from({length: board.width}).map((__, x) => (
              <div key={`${x}-${y}`}
                   onDragOver={(e)=>e.preventDefault()}
                   onDrop={(e)=>onDropCell(x, y, e)}
                   style={{width: 28, height: 28, border: '1px solid #ccc', display:'flex', alignItems:'center', justifyContent:'center', background:'#fff'}}>
                {cellDisplay(x,y).slice(0,1)}
              </div>
            ))
          ))}
        </div>
        <div>
          <div style={{marginBottom: 6, fontSize: 12, opacity: 0.7}}>Rack (drag onto board)</div>
          <div style={{display:'flex', gap: 6}}>
            {rack.map((k, i) => (
              <div key={i} draggable onDragStart={(e)=>onDragStartTile(k, e)}
                   style={{width:28, height:28, border:'1px solid #aaa', display:'flex', alignItems:'center', justifyContent:'center', background:'#f9f9f9', cursor:'grab'}}>
                {k}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
