import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';

type BoardJson = { width: number; height: number; rows: string[][] };
type Placement = { x: number; y: number; kind_id: string };

export default function Playground(): JSX.Element {
  const [ready, setReady] = useState(false);
  const [useWorker, setUseWorker] = useState(false);
  const [useDict, setUseDict] = useState(true);
  const [dictEngine, setDictEngine] = useState<'fst' | 'set' | 'dawg' | 'gaddag'>('fst');
  const [useAnagram, setUseAnagram] = useState(false);
  const [useHex, setUseHex] = useState(false);
  const [useDiag, setUseDiag] = useState(false);
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
    if (useDiag) {
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
          tryEdge(x,y,x+1,y,'E');
          tryEdge(x,y,x-1,y,'W');
          tryEdge(x,y,x,y-1,'N');
          tryEdge(x,y,x,y+1,'S');
          tryEdge(x,y,x+1,y-1,'NE');
          tryEdge(x,y,x-1,y-1,'NW');
          tryEdge(x,y,x+1,y+1,'SE');
          tryEdge(x,y,x-1,y+1,'SW');
        }
      }
      return { ...base, board_layout: { width, height, type: 'graph', nodes, edges } };
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
  }, [useHex, useDiag, use3D, depth]);

  // Optional: build a tiny anagram index from the demo dictionary
  const [anagramIndex, setAnagramIndex] = useState<Map<string, string> | null>(null);
  useEffect(() => {
    let cancelled = false;
    async function loadIndex() {
      if (!useAnagram || !useDict) { setAnagramIndex(null); return; }
      try {
        // Prefer text to avoid bundling large FST parsing on the client
        const resp = await fetch('/dictionaries/TWL06.txt');
        if (!resp.ok) { setAnagramIndex(null); return; }
        const txt = await resp.text();
        // Build a small index only for words length <= 7 (rack size)
        const m = new Map<string, string>();
        const maxLen = 7;
        for (const raw of txt.split(/\r?\n/)) {
          const w = raw.trim();
          if (!w || w.startsWith('#')) continue;
          if (w.length > maxLen) continue;
          const sig = w.toUpperCase().split('').sort().join('');
          if (!m.has(sig)) m.set(sig, w.toUpperCase());
        }
        if (!cancelled) setAnagramIndex(m);
      } catch {
        if (!cancelled) setAnagramIndex(null);
      }
    }
    loadIndex();
    return () => { cancelled = true; };
  }, [useAnagram, useDict]);

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
        if (useDict) {
          try {
            if (dictEngine === 'fst') {
              const resp = await fetch('/dictionaries/TWL06.fst');
              if (resp.ok) {
                const buf = new Uint8Array(await resp.arrayBuffer());
                await call('set_dictionary_from_fst_bytes', { bytes: buf, case_fold: true });
              } else {
                const txtResp = await fetch('/dictionaries/TWL06.txt');
                if (txtResp.ok) {
                  const txt = await txtResp.text();
                  await call('set_dictionary_from_text', { text: txt, case_fold: true });
                }
              }
            } else {
              const txtResp = await fetch('/dictionaries/TWL06.txt');
              if (txtResp.ok) {
                const txt = await txtResp.text();
                await call('set_dictionary_engine', { text: txt, engine: dictEngine, case_fold: true });
              }
            }
          } catch (e) {
            console.error('Failed to load dictionary', e);
          }
        }
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
            if (dictEngine === 'fst') {
              const resp = await fetch('/dictionaries/TWL06.fst');
              if (resp.ok) {
                const buf = new Uint8Array(await resp.arrayBuffer());
                mod.set_dictionary_from_fst_bytes(g, buf, true);
              } else {
                const txtResp = await fetch('/dictionaries/TWL06.txt');
                if (txtResp.ok) {
                  const txt = await txtResp.text();
                  mod.set_dictionary_from_text(g, txt, true);
                }
              }
            } else {
              const txtResp = await fetch('/dictionaries/TWL06.txt');
              if (txtResp.ok) {
                const txt = await txtResp.text();
                mod.set_dictionary_from_text_engine(g, txt, dictEngine, true);
              }
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
  }, [useWorker, useDict, cfg, rtl, stackOn, stackScoring, forbidSame, dictEngine]);

  const rack = useMemo(() => ['A','A','A','B','B'], []);

  const commit = async () => {
    if (!game || pending.length === 0) return;
    try {
      let placements = pending;
      // If anagram mode is enabled and we have an index, try to reorder letters to form any dictionary word
      if (useAnagram && useDict && anagramIndex && board) {
        // Check if placements are on a straight line (row or column)
        const allX = new Set(placements.map(p => p.x));
        const allY = new Set(placements.map(p => p.y));
        const isRow = allY.size === 1;
        const isCol = allX.size === 1;
        if (isRow || isCol) {
          // Derive the current letters from pending or board (pending contains only kind_id)
          const letters = placements.map(p => p.kind_id.toUpperCase());
          const sig = letters.slice().sort().join('');
          const word = anagramIndex.get(sig);
          if (word) {
            // Order the placements along the line and assign letters from the found word
            const sorted = [...placements].sort((a,b) => (isRow ? a.x - b.x : a.y - b.y));
            placements = sorted.map((p, i) => ({ ...p, kind_id: word[i] }));
          }
        }
      }
      if (useWorker) {
        await game.call('play_move', { placements });
        const { board: b } = await game.call('get_board');
        setBoard(JSON.parse(b as string) as BoardJson);
      } else {
        game.mod.play_move(game.g, JSON.stringify(placements));
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
        <label><input type="checkbox" checked={useHex} onChange={e => { setUseHex(e.target.checked); setUseDiag(false); setUse3D(false); }} /> Hex adjacency</label>
        <label><input type="checkbox" checked={useDiag} onChange={e => { setUseDiag(e.target.checked); setUseHex(false); setUse3D(false); }} /> Diagonal adjacency</label>
        <label><input type="checkbox" checked={use3D} onChange={e => { setUse3D(e.target.checked); setUseHex(false); }} /> 3D (layers)</label>
        {use3D && <>
          <label>Depth: <input type="number" min={1} max={9} value={depth} onChange={e => { const v = Math.max(1, Math.min(9, parseInt(e.target.value||'1'))); setDepth(v); setZ(0); }} style={{width:50}}/></label>
          <label>Slice z: <input type="range" min={0} max={Math.max(0, depth-1)} value={z} onChange={e => setZ(parseInt(e.target.value))} /></label>
        </>}
        <label><input type="checkbox" checked={useDict} onChange={e => setUseDict(e.target.checked)} /> Dictionary checks (TWL06)</label>
        <label>Engine:
          <select value={dictEngine} onChange={e => setDictEngine(e.target.value as 'fst' | 'set' | 'dawg' | 'gaddag')} disabled={!useDict}>
            <option value="fst">FST</option>
            <option value="set">Set</option>
            <option value="dawg">DAWG</option>
            <option value="gaddag">GADDAG</option>
          </select>
        </label>
        <label title="Reorder placed tiles to any valid anagram on commit (row/column only)"><input type="checkbox" checked={useAnagram} onChange={e => setUseAnagram(e.target.checked)} /> Anagram Mode</label>
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
