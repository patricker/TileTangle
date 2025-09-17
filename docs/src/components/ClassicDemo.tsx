import React, {useEffect, useMemo, useRef, useState} from 'react';
import {classicTilesets, classicBonuses} from './demoUtils';

type BoardJson = { width: number; height: number; rows: string[][] };
type Placement = { x: number; y: number; kind_id: string; mark?: string|null };

type WasmGame = {
  call: (action: string, payload?: any) => Promise<any>;
};

export default function ClassicDemo(): JSX.Element {
  const [game, setGame] = useState<WasmGame|null>(null);
  const [board, setBoard] = useState<BoardJson|null>(null);
  const [rack, setRack] = useState<{kind_id:string;mark?:string|null}[]>([]);
  const [scores, setScores] = useState<number[]>([]);
  const [useDict, setUseDict] = useState(false);
  const [rtl, setRtl] = useState(false);
  const [stackOn, setStackOn] = useState(false);
  const [stackScoring, setStackScoring] = useState<'top'|'sum'>('top');
  const [forbidSame, setForbidSame] = useState(true);
  const [pending, setPending] = useState<Placement[]>([]);
  const [exchangeSel, setExchangeSel] = useState<Set<number>>(new Set());
  const [showHints, setShowHints] = useState(false);
  const [hints, setHints] = useState<{placements: Placement[]; score:number; word:string}[]>([]);
  const workerRef = useRef<Worker|null>(null);

  const cfg = useMemo(() => {
    const { tile_kinds, tile_counts } = classicTilesets();
    return {
      tileset: { tile_kinds },
      rack_size: 7,
      board_layout: { width: 15, height: 15 },
      ruleset_id: 'cross',
      dictionary_id: 'en',
      rng_seed: 42,
      tile_counts,
      free_word_mode: !useDict,
    } as any;
  }, [useDict]);

  useEffect(() => {
    (async () => {
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
      await call('new_game', { config: cfg, players: 2 });
      await call('set_reading_direction', { rtl });
      await call('set_stacking', { enabled: stackOn, max_height: 7, forbid_same: forbidSame, scoring: stackScoring });
      await call('set_bonuses', classicBonuses());
      if (useDict) {
        try {
          const resp = await fetch('/dictionaries/TWL06.fst');
          if (resp.ok) {
            const buf = new Uint8Array(await resp.arrayBuffer());
            await call('set_dictionary_from_fst_bytes', { bytes: buf, case_fold: true });
          } else {
            const txtResp = await fetch('/dictionaries/TWL06.txt');
            const txt = await txtResp.text();
            await call('set_dictionary_from_text', { text: txt, case_fold: true });
          }
        } catch (e) { console.error('Failed to load dictionary', e); }
      }
      const { board: b } = await call('get_board');
      const { rack: r } = await call('get_rack');
      const { scores: s } = await call('get_scores');
      setGame({ call });
      setBoard(JSON.parse(b as string) as BoardJson);
      setRack(JSON.parse(r as string));
      setScores(JSON.parse(s as string));
    })();
    return () => {
      workerRef.current?.terminate();
      workerRef.current = null;
    };
  }, [cfg, rtl, stackOn, stackScoring, forbidSame]);

  const commit = async () => {
    if (!game || pending.length === 0) return;
    try {
      const res = await game.call('play_move', { placements: pending });
      const { board: b } = await game.call('get_board');
      const { rack: r } = await game.call('get_rack');
      const { scores: s } = await game.call('get_scores');
      setPending([]);
      setBoard(JSON.parse(b as string) as BoardJson);
      setRack(JSON.parse(r as string));
      setScores(JSON.parse(s as string));
    } catch (e) { console.error(e); }
  };

  const pass = async () => {
    if (!game) return;
    await game.call('pass_turn');
    const { board: b } = await game.call('get_board');
    const { rack: r } = await game.call('get_rack');
    const { scores: s } = await game.call('get_scores');
    setBoard(JSON.parse(b as string) as BoardJson);
    setRack(JSON.parse(r as string));
    setScores(JSON.parse(s as string));
    setPending([]);
    setExchangeSel(new Set());
  };

  const exchange = async () => {
    if (!game || exchangeSel.size === 0) return;
    const kinds = Array.from(exchangeSel).map(i => rack[i].kind_id);
    await game.call('exchange_tiles', { kinds });
    const { board: b } = await game.call('get_board');
    const { rack: r } = await game.call('get_rack');
    const { scores: s } = await game.call('get_scores');
    setBoard(JSON.parse(b as string) as BoardJson);
    setRack(JSON.parse(r as string));
    setScores(JSON.parse(s as string));
    setExchangeSel(new Set());
    setPending([]);
  };

  useEffect(() => {
    (async () => {
      if (!game || !showHints) { setHints([]); return; }
      try {
        const { moves } = await game.call('generate_moves', { max_len: 15, limit: 5 });
        const arr = JSON.parse(moves as string) as { placements: Placement[]; score:number; word:string }[];
        setHints(arr);
      } catch (e) { console.error(e); }
    })();
  }, [game, showHints, board]);

  const aiMove = async () => {
    if (!game) return;
    try {
      const { moves } = await game.call('generate_moves', { max_len: 15, limit: 1 });
      const arr = JSON.parse(moves as string) as { placements: Placement[] }[];
      if (arr.length === 0) return;
      await game.call('play_move', { placements: arr[0].placements });
      const { board: b } = await game.call('get_board');
      const { rack: r } = await game.call('get_rack');
      const { scores: s } = await game.call('get_scores');
      setBoard(JSON.parse(b as string) as BoardJson);
      setRack(JSON.parse(r as string));
      setScores(JSON.parse(s as string));
    } catch (e) { console.error(e); }
  };

  const playSuggested = async (idx: number) => {
    if (!game || idx < 0 || idx >= hints.length) return;
    const h = hints[idx];
    await game.call('play_move', { placements: h.placements });
    const { board: b } = await game.call('get_board');
    const { rack: r } = await game.call('get_rack');
    const { scores: s } = await game.call('get_scores');
    setBoard(JSON.parse(b as string) as BoardJson);
    setRack(JSON.parse(r as string));
    setScores(JSON.parse(s as string));
  };

  const onDropCell = (x: number, y: number, ev: React.DragEvent<HTMLDivElement>) => {
    ev.preventDefault();
    const data = ev.dataTransfer.getData('text/plain');
    if (!data) return;
    const [kind_id, is_blank] = data.split(':');
    const mark = (is_blank === '1') ? (window.prompt('Blank tile: enter symbol', 'A') || '').toUpperCase() : null;
    setPending(prev => {
      if (prev.some(p => p.x === x && p.y === y)) return prev;
      const id = { x, y, kind_id, mark } as Placement;
      return [...prev, id];
    });
  };

  const onDragStartTile = (k: {kind_id:string;mark?:string|null}, ev: React.DragEvent<HTMLDivElement>) => {
    const isBlank = k.kind_id === 'BL' ? '1' : '0';
    ev.dataTransfer.setData('text/plain', `${k.kind_id}:${isBlank}`);
  };

  const letterAt = (x: number, y: number): string => {
    if (!board) return '';
    const s = board.rows[y][x] || '';
    const pend = pending.find(p => p.x === x && p.y === y);
    return pend ? pend.kind_id : s;
  };

  if (!board || !game) return <div>Loading…</div>;

  const hintIndexAt = (x:number,y:number): number => {
    for (let i=0;i<Math.min(hints.length,5);i++) {
      const h = hints[i];
      if (h.placements.some(p => p.x===x && p.y===y)) return i+1;
    }
    return 0;
  };

  return (
    <div>
      <div style={{display:'flex', gap:12, alignItems:'center', marginBottom: 12}}>
        <label><input type="checkbox" checked={useDict} onChange={e => setUseDict(e.target.checked)} /> Dictionary checks</label>
        <label><input type="checkbox" checked={rtl} onChange={e => setRtl(e.target.checked)} /> RTL</label>
        <label><input type="checkbox" checked={stackOn} onChange={e => setStackOn(e.target.checked)} /> Stacking</label>
        {stackOn && (
          <>
            <label>Scoring:
              <select value={stackScoring} onChange={e => setStackScoring(e.target.value as any)}>
                <option value="top">TopOnly</option>
                <option value="sum">SumStack</option>
              </select>
            </label>
            <label><input type="checkbox" checked={forbidSame} onChange={e => setForbidSame(e.target.checked)} /> Forbid same overlay</label>
          </>
        )}
        <button onClick={commit} disabled={pending.length===0}>Commit ({pending.length})</button>
        <button onClick={pass}>Pass</button>
        <button onClick={async()=>{ await game?.call('undo'); const { board: b } = await game!.call('get_board'); const { rack: r } = await game!.call('get_rack'); const { scores: s } = await game!.call('get_scores'); setBoard(JSON.parse(b as string)); setRack(JSON.parse(r as string)); setScores(JSON.parse(s as string)); }}>Undo</button>
        <button onClick={async()=>{ await game?.call('redo'); const { board: b } = await game!.call('get_board'); const { rack: r } = await game!.call('get_rack'); const { scores: s } = await game!.call('get_scores'); setBoard(JSON.parse(b as string)); setRack(JSON.parse(r as string)); setScores(JSON.parse(s as string)); }}>Redo</button>
        <button onClick={exchange} disabled={exchangeSel.size===0}>Exchange ({exchangeSel.size})</button>
        <button onClick={()=>setPending([])} disabled={pending.length===0}>Reset Pending</button>
        <button onClick={aiMove}>AI Move</button>
        <label><input type="checkbox" checked={showHints} onChange={e=>setShowHints(e.target.checked)} /> Show Hints</label>
        <div style={{marginLeft:'auto'}}>Scores: {scores.join(' : ')}</div>
      </div>
      <div style={{display:'flex', gap:16}}>
        <div>
          <div style={{display:'grid', gridTemplateColumns:`repeat(${board.width}, 28px)`, gap:4}}>
            {Array.from({length: board.height}).map((_, y) => (
              Array.from({length: board.width}).map((__, x) => (
                <div key={`${x}-${y}`}
                     onDragOver={e=>e.preventDefault()}
                     onDrop={e=>onDropCell(x,y,e)}
                     data-testid="classic-board-cell"
                     data-x={x}
                     data-y={y}
                     style={{width:28, height:28, border:'1px solid #ccc', background:'#fff', display:'flex', alignItems:'center', justifyContent:'center', fontSize:12, position:'relative'}}>
                  {letterAt(x,y)}
                  {showHints && (()=>{ const idx = hintIndexAt(x,y); return idx>0 ? <div style={{position:'absolute', inset:2, background:`rgba(255,165,0,0.25)`, color:'#b55', fontSize:9, display:'flex', alignItems:'center', justifyContent:'center'}}>{idx}</div> : null })()}
                </div>
              ))
            ))}
          </div>
        </div>
        <div>
          <div style={{marginBottom:6, fontSize:12, opacity:0.7}}>Rack (drag)</div>
          <div style={{display:'flex', gap:6}}>
            {rack.map((t, i) => {
              const sel = exchangeSel.has(i);
              return (
              <div
                key={i}
                draggable
                data-testid="classic-rack-tile"
                data-kind={t.kind_id}
                onDragStart={(e)=>onDragStartTile(t, e)}
                onClick={()=>{
                    const ns = new Set(exchangeSel); sel ? ns.delete(i) : ns.add(i); setExchangeSel(ns);
                  }}
                   style={{width:28, height:28, border:'1px solid #aaa', display:'flex', alignItems:'center', justifyContent:'center', background: sel?'#cfe8ff':'#f9f9f9', cursor:'grab'}}>
                {t.kind_id}
              </div>
            )})}
          </div>
        </div>
        {showHints && (
          <div style={{minWidth: 180}}>
            <div style={{marginBottom:6, fontSize:12, opacity:0.7}}>Top Moves</div>
            {hints.map((h, i) => (
              <div key={i} style={{display:'flex', alignItems:'center', justifyContent:'space-between', marginBottom:4}}>
                <div>#{i+1} {h.word} (+{h.score})</div>
                <button onClick={()=>playSuggested(i)}>Play</button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
