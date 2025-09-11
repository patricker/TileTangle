import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';

type BoardJson = { width: number; height: number; rows: string[][] };
type Placement = { x: number; y: number; kind_id: string };

export default function Playground(): JSX.Element {
  const [ready, setReady] = useState(false);
  const [useWorker, setUseWorker] = useState(false);
  const [useDict, setUseDict] = useState(true);
  const [useHex, setUseHex] = useState(false);
  const [use3D, setUse3D] = useState(false);
  const [rtl, setRtl] = useState(false);
  const [stackOn, setStackOn] = useState(false);
  const [stackScoring, setStackScoring] = useState<'top'|'sum'>('top');
  const [forbidSame, setForbidSame] = useState(true);
  const [depth, setDepth] = useState(3);
  const [z, setZ] = useState(0);
  const [game, setGame] = useState<any>(null);
  const [board, setBoard] = useState<BoardJson | null>(null);
  const [pending, setPending] = useState<Placement[]>([]);
  const workerRef = useRef<Worker | null>(null);

  const cfg = useMemo(() => {
    const width = 9, height = 9;
    const base = {
      tileset: { tile_kinds: [
        { id: 'A', symbol: 'A', score: 1 },
        { id: 'B', symbol: 'B', score: 3 },
      ] },
      rack_size: 7,
      ruleset_id: 'cross', dictionary_id: 'en', rng_seed: 1,
      tile_counts: { A: 30, B: 12 }, free_word_mode: true,
    } as any;
    if (use3D) {
      return { ...base, board_layout: { type: '3d', width, height, depth } } as any;
    }
    if (!useHex) return { ...base, board_layout: { width, height } };
    // Build hex-style adjacency on a rectangular grid (even-r offset)
    const nodes: {x:number;y:number}[] = [];
    for (let y=0;y<height;y++) for (let x=0;x<width;x++) nodes.push({x,y});
    const index = (x:number,y:number) => y*width + x;
    const edges: {a:number;b:number;dir:string}[] = [];
    const tryEdge = (x1:number,y1:number,x2:number,y2:number,dir:string) => {
      if (x2<0||x2>=width||y2<0||y2>=height) return;
      edges.push({ a:index(x1,y1), b:index(x2,y2), dir });
    };
    for (let y=0;y<height;y++) {
      for (let x=0;x<width;x++) {
        const even = (y % 2) === 0;
        // Use only forward directions; overlay builds reverse links
        tryEdge(x,y,x+1,y,'E');
        // NE and SE (reverse links provide NW/SW)
        tryEdge(x,y, x + (even?0:1), y-1, 'NE');
        tryEdge(x,y, x + (even?0:1), y+1, 'SE');
      }
    }
    return { ...base, board_layout: { width, height, type: 'graph', nodes, edges } };
  }, [useHex, use3D, depth]);

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
        await call('set_reading_direction', { rtl });
        await call('set_stacking', { enabled: stackOn, max_height: 7, forbid_same: forbidSame, scoring: stackScoring });
        await call('set_free_word_mode', { on: !useDict });
        const { board: b } = await call('get_board');
        setGame({ call });
        setBoard(JSON.parse(b as string) as BoardJson);
        setReady(true);
      } else {
        const mod = await import('/wasm/engine/pkg/tiletangle_wasm.js');
        await mod.default();
        const cfg2 = { ...cfg, free_word_mode: !useDict } as any;
        const g = mod.new_game(JSON.stringify(cfg2), 2);
        mod.set_reading_direction(g, rtl);
        mod.set_stacking(g, stackOn, 7, forbidSame, stackScoring === 'sum');
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
  }, [useWorker, useDict, cfg, rtl, stackOn, stackScoring, forbidSame]);

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
      if (board) {
        const h = use3D ? Math.floor(board.height / depth) : board.height;
        const gy = use3D ? (y + z * h) : y;
        if (!stackOn && (board.rows[gy][x] || '').length > 0) return prev;
        return [...prev, { x, y: gy, kind_id }];
      }
      return prev;
    });
  };

  const onDragStartTile = (k: string, ev: React.DragEvent<HTMLDivElement>) => {
    ev.dataTransfer.setData('text/plain', k);
  };

  const cellDisplay = (x: number, y: number): string => {
    const p = pending.find(pp => pp.x === x && pp.y === y);
    if (p) return p.kind_id;
    if (use3D && board) {
      const h = Math.floor(board.height / depth);
      const yy = y + z * h;
      return (board.rows[yy][x] || '');
    }
    return (board?.rows[y][x] || '');
  };

  if (!ready || !board) return <div>Loading WASM…</div>;

  return (
    <div>
      <div style={{display:'flex', alignItems:'center', gap:12, marginBottom: 12}}>
        <label><input type="checkbox" checked={useWorker} onChange={e => setUseWorker(e.target.checked)} /> Use Web Worker</label>
        <label><input type="checkbox" checked={useHex} onChange={e => { setUseHex(e.target.checked); setUse3D(false); }} /> Hex adjacency</label>
        <label><input type="checkbox" checked={use3D} onChange={e => { setUse3D(e.target.checked); setUseHex(false); }} /> 3D (layers)</label>
        {use3D && <>
          <label>Depth: <input type="number" min={1} max={9} value={depth} onChange={e => { const v = Math.max(1, Math.min(9, parseInt(e.target.value||'1'))); setDepth(v); setZ(0); }} style={{width:50}}/></label>
          <label>Slice z: <input type="range" min={0} max={Math.max(0, depth-1)} value={z} onChange={e => setZ(parseInt(e.target.value))} /></label>
        </>}
        <label><input type="checkbox" checked={useDict} onChange={e => setUseDict(e.target.checked)} /> Dictionary checks (TWL06)</label>
        <label><input type="checkbox" checked={rtl} onChange={e => setRtl(e.target.checked)} /> RTL reading</label>
        <label><input type="checkbox" checked={stackOn} onChange={e => setStackOn(e.target.checked)} /> Stacking</label>
        {stackOn && (<>
          <label>Scoring: 
            <select value={stackScoring} onChange={e => setStackScoring((e.target.value as any))}>
              <option value="top">TopOnly</option>
              <option value="sum">SumStack</option>
            </select>
          </label>
          <label><input type="checkbox" checked={forbidSame} onChange={e => setForbidSame(e.target.checked)} /> Forbid same overlay</label>
        </>)}
        <button onClick={commit} disabled={pending.length === 0}>Commit Move ({pending.length})</button>
        <button onClick={() => setPending([])} disabled={pending.length === 0}>Reset</button>
      </div>
      <div style={{display:'flex', gap: 16, alignItems:'flex-start'}}>
        <div style={{display: 'grid', gridTemplateColumns: `repeat(${board.width}, 28px)`, gap: 4}}>
          {Array.from({length: use3D ? Math.floor(board.height / depth) : board.height}).map((_, y) => (
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
