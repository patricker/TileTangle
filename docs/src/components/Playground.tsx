import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';

type BoardJson = { width: number; height: number; rows: string[][] };
type Placement = { x: number; y: number; kind_id: string; mark?: string | null };
type GeneratedMove = {
  word: string;
  score: number;
  placements: { x: number; y: number; kind_id: string; mark?: string | null }[];
};

type AiSuggestion = {
  difficulty: string;
  word: string;
  score: number;
  total: number;
  rackLeave: number;
  boardEquity: number;
  endgamePenalty: number;
  placements: Placement[];
};

type PlaygroundInitialConfig = {
  tileset?: { tile_kinds: { id: string; symbol: string; score: number; is_blank?: boolean; aliases?: string[] }[] };
  tile_counts?: Record<string, number>;
  rack_size?: number;
  board_layout?: Record<string, unknown>;
  ruleset_id?: string;
  dictionary_id?: string;
  rng_seed?: number;
};

type PlaygroundInitial = {
  useWorker?: boolean;
  useDict?: boolean;
  dictEngine?: 'fst' | 'set' | 'dawg' | 'gaddag';
  useAnagram?: boolean;
  useHex?: boolean;
  useDiag?: boolean;
  use3D?: boolean;
  depth?: number;
  rtl?: boolean;
  stackOn?: boolean;
  stackScoring?: 'top' | 'sum';
  forbidSame?: boolean;
  cpuDifficulty?: 'off' | 'easy' | 'medium' | 'hard';
  config?: PlaygroundInitialConfig;
  rack?: string[];
};

type PlaygroundProps = {
  initial?: PlaygroundInitial;
};

export default function Playground({initial}: PlaygroundProps = {}): JSX.Element {
  const configOverrideRef = useRef<PlaygroundInitialConfig | undefined>(initial?.config);
  const rackOverrideRef = useRef<string[] | undefined>(initial?.rack);

  const [ready, setReady] = useState(false);
  const [useWorker, setUseWorker] = useState(initial?.useWorker ?? false);
  const [useDict, setUseDict] = useState(initial?.useDict ?? true);
  const [dictEngine, setDictEngine] = useState<'fst' | 'set' | 'dawg' | 'gaddag'>(initial?.dictEngine ?? 'fst');
  const [useAnagram, setUseAnagram] = useState(initial?.useAnagram ?? false);
  const [useHex, setUseHex] = useState(initial?.useHex ?? false);
  const [useDiag, setUseDiag] = useState(initial?.useDiag ?? false);
  const [use3D, setUse3D] = useState(initial?.use3D ?? false);
  const [rtl, setRtl] = useState(initial?.rtl ?? false);
  const [stackOn, setStackOn] = useState(initial?.stackOn ?? false);
  const [stackScoring, setStackScoring] = useState<'top'|'sum'>(initial?.stackScoring ?? 'top');
  const [forbidSame, setForbidSame] = useState(initial?.forbidSame ?? true);
  const [depth, setDepth] = useState(initial?.depth ?? 3);
  const [z, setZ] = useState(0);
  const [game, setGame] = useState<any>(null);
  const [board, setBoard] = useState<BoardJson | null>(null);
  const [pending, setPending] = useState<Placement[]>([]);
  const [showMoves, setShowMoves] = useState(false);
  const [legalMoves, setLegalMoves] = useState<GeneratedMove[]>([]);
  const [activeMoveIndex, setActiveMoveIndex] = useState<number | null>(null);
  const [loadingMoves, setLoadingMoves] = useState(false);
  const [rack, setRack] = useState<string[]>(rackOverrideRef.current ?? []);
  const [cpuDifficulty, setCpuDifficulty] = useState<'off' | 'easy' | 'medium' | 'hard'>(initial?.cpuDifficulty ?? 'off');
  const [cpuThinking, setCpuThinking] = useState(false);
  const [cpuSuggestion, setCpuSuggestion] = useState<AiSuggestion | null>(null);
  const [lastCpu, setLastCpu] = useState<AiSuggestion | null>(null);
  const [snapshotText, setSnapshotText] = useState('');
  const [eventLogText, setEventLogText] = useState('');
  const workerRef = useRef<Worker | null>(null);

  const defaultTileset = useMemo(() => ({
    tile_kinds: [
      { id: 'A', symbol: 'A', score: 1 },
      { id: 'B', symbol: 'B', score: 3 },
    ],
  }), []);

  const defaultTileCounts = useMemo(() => ({ A: 30, B: 12 }), []);

  const cfg = useMemo(() => {
    const override = configOverrideRef.current;
    const baseWidth = typeof override?.board_layout === 'object' && override?.board_layout !== null && 'width' in (override.board_layout as any)
      ? Number((override.board_layout as any).width)
      : 9;
    const baseHeight = typeof override?.board_layout === 'object' && override?.board_layout !== null && 'height' in (override.board_layout as any)
      ? Number((override.board_layout as any).height)
      : 9;

    const width = Number.isFinite(baseWidth) && baseWidth > 0 ? baseWidth : 9;
    const height = Number.isFinite(baseHeight) && baseHeight > 0 ? baseHeight : 9;

    let boardLayout: Record<string, unknown>;
    if (override?.board_layout) {
      boardLayout = override.board_layout;
    } else if (use3D) {
      boardLayout = { type: '3d', width, height, depth };
    } else if (useDiag) {
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
      boardLayout = { width, height, type: 'graph', nodes, edges };
    } else if (useHex) {
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
          tryEdge(x,y,x+1,y,'E');
          tryEdge(x,y, x + (even ? 0 : 1), y-1, 'NE');
          tryEdge(x,y, x + (even ? 0 : 1), y+1, 'SE');
        }
      }
      boardLayout = { width, height, type: 'graph', nodes, edges };
    } else {
      boardLayout = { width, height };
    }

    return {
      tileset: override?.tileset ?? defaultTileset,
      rack_size: override?.rack_size ?? 7,
      board_layout: boardLayout,
      ruleset_id: override?.ruleset_id ?? 'cross',
      dictionary_id: override?.dictionary_id ?? 'en',
      rng_seed: override?.rng_seed ?? 1,
      tile_counts: override?.tile_counts ?? defaultTileCounts,
      free_word_mode: !useDict,
    } as any;
  }, [useDict, useHex, useDiag, use3D, depth, defaultTileset, defaultTileCounts]);

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
        const overrideRack = rackOverrideRef.current;
        if (overrideRack && overrideRack.length) {
          await call('set_rack', { tiles: overrideRack });
        }
        const { board: b } = await call('get_board');
        const { rack: r } = await call('get_rack');
        setGame({ call });
        setBoard(JSON.parse(b as string) as BoardJson);
        setRack(JSON.parse(r as string).map((t: any) => t.kind_id));
        setLastCpu(null);
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
        const overrideRack = rackOverrideRef.current;
        if (overrideRack && overrideRack.length) {
          mod.set_rack(g, JSON.stringify(overrideRack));
        }
        setGame({ mod, g });
        setBoard(JSON.parse(mod.get_board(g)) as BoardJson);
        setRack(
          (JSON.parse(mod.get_rack(g)) as { kind_id: string }[]).map((t) => t.kind_id),
        );
        setLastCpu(null);
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

  const clearMoves = useCallback(() => {
    setShowMoves(false);
    setLegalMoves([]);
    setActiveMoveIndex(null);
  }, []);

  const refreshRack = useCallback(async () => {
    if (!game) {
      setRack([]);
      return;
    }
    try {
      if (useWorker) {
        const resp = await game.call('get_rack');
        const arr = JSON.parse(resp.rack as string) as { kind_id: string }[];
        setRack(arr.map(t => t.kind_id));
      } else {
        const arr = JSON.parse(game.mod.get_rack(game.g)) as { kind_id: string }[];
        setRack(arr.map(t => t.kind_id));
      }
    } catch (err) {
      console.error('get_rack failed', err);
    }
  }, [game, useWorker]);

  const syncBoard = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        const { board: b } = await game.call('get_board');
        setBoard(JSON.parse(b as string) as BoardJson);
      } else {
        setBoard(JSON.parse(game.mod.get_board(game.g)) as BoardJson);
      }
      await refreshRack();
      setPending([]);
      clearMoves();
      setCpuSuggestion(null);
      setLastCpu(null);
    } catch (err) {
      console.error('sync board failed', err);
    }
  }, [game, useWorker, refreshRack, clearMoves]);

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
      await refreshRack();
      setLastCpu(null);
      setPending([]);
      clearMoves();
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

  const highlightCells = useMemo(() => {
    if (activeMoveIndex === null) return new Set<string>();
    const mv = legalMoves[activeMoveIndex];
    if (!mv) return new Set<string>();
    const set = new Set<string>();
    mv.placements.forEach(p => set.add(`${p.x},${p.y}`));
    return set;
  }, [activeMoveIndex, legalMoves]);

  const fetchMoves = useCallback(async () => {
    if (!game) return;
    setLoadingMoves(true);
    try {
      let moves: GeneratedMove[] = [];
      const maxLen = 7;
      if (useWorker) {
        const resp = await game.call('generate_moves', { max_len: maxLen, limit: 20 });
        moves = JSON.parse(resp.moves as string) as GeneratedMove[];
      } else {
        const json = game.mod.generate_moves(game.g, maxLen, 20);
        moves = JSON.parse(json) as GeneratedMove[];
      }
      setLegalMoves(moves);
      setActiveMoveIndex(moves.length ? 0 : null);
      setShowMoves(true);
    } catch (err) {
      console.error('generate_moves failed', err);
      setLegalMoves([]);
      setActiveMoveIndex(null);
      setShowMoves(true);
    } finally {
      setLoadingMoves(false);
    }
  }, [game, useWorker, use3D, depth]);

  const playGeneratedMove = useCallback(async (move: GeneratedMove) => {
    if (!game) return;
    try {
      if (useWorker) {
        await game.call('play_move', { placements: move.placements });
        const { board: b } = await game.call('get_board');
        setBoard(JSON.parse(b as string) as BoardJson);
      } else {
        game.mod.play_move(game.g, JSON.stringify(move.placements));
        setBoard(JSON.parse(game.mod.get_board(game.g)) as BoardJson);
      }
      await refreshRack();
      setPending([]);
      clearMoves();
    } catch (err) {
      console.error('play_generated_move failed', err);
    }
  }, [game, useWorker, refreshRack, clearMoves]);

  const requestCpuHint = useCallback(async () => {
    if (!game || cpuDifficulty === 'off') {
      setCpuSuggestion(null);
      return;
    }
    setCpuThinking(true);
    try {
      let bestJson: string;
      if (useWorker) {
        const resp = await game.call('best_move', {
          difficulty: cpuDifficulty,
          seed: 42,
        });
        bestJson = resp.best as string;
      } else {
        bestJson = game.mod.best_move(game.g, cpuDifficulty, 42);
      }
      const payload = JSON.parse(bestJson);
      const placements: Placement[] = (payload.placements || []).map((p: any) => ({
        x: p.x,
        y: p.y,
        kind_id: p.kind_id,
        mark: p.mark ?? null,
      }));
      const suggestion: AiSuggestion = {
        difficulty: cpuDifficulty,
        word: payload.word,
        score: payload.score,
        total: payload.total,
        rackLeave: payload.rack_leave,
        boardEquity: payload.board_equity,
        endgamePenalty: payload.endgame_penalty,
        placements,
      };
      setCpuSuggestion(suggestion);
      setLastCpu(suggestion);
    } catch (err) {
      console.error('best_move failed', err);
      setCpuSuggestion(null);
    } finally {
      setCpuThinking(false);
    }
  }, [game, useWorker, cpuDifficulty]);

  const playCpuSuggestion = useCallback(async () => {
    if (!cpuSuggestion) return;
    await playGeneratedMove({
      word: cpuSuggestion.word,
      score: cpuSuggestion.score,
      placements: cpuSuggestion.placements,
    });
    setLastCpu(cpuSuggestion);
    setCpuSuggestion(null);
  }, [cpuSuggestion, playGeneratedMove]);

  useEffect(() => {
    if (cpuDifficulty === 'off') {
      setCpuSuggestion(null);
    }
  }, [cpuDifficulty]);

  const exportSnapshot = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        const resp = await game.call('snapshot_json');
        setSnapshotText(String((resp as any)?.snapshot ?? ''));
      } else {
        const snap = game.mod.snapshot_state_json(game.g);
        setSnapshotText(snap);
      }
    } catch (err) {
      console.error('snapshot export failed', err);
    }
  }, [game, useWorker]);

  const importSnapshot = useCallback(async () => {
    if (!game || !snapshotText.trim()) return;
    try {
      if (useWorker) {
        await game.call('load_snapshot_json', { json: snapshotText });
        const { board: b } = await game.call('get_board');
        setBoard(JSON.parse(b as string) as BoardJson);
      } else {
        game.mod.load_state_json(game.g, snapshotText);
        setBoard(JSON.parse(game.mod.get_board(game.g)) as BoardJson);
      }
      await refreshRack();
      setPending([]);
      clearMoves();
      setCpuSuggestion(null);
      setLastCpu(null);
    } catch (err) {
      console.error('snapshot import failed', err);
    }
  }, [game, snapshotText, useWorker, refreshRack, clearMoves]);

  const fetchEventLog = useCallback(async () => {
    if (!game) return;
    try {
      let payload: string;
      if (useWorker) {
        const resp = await game.call('event_log');
        payload = String((resp as any)?.log ?? '[]');
      } else {
        payload = game.mod.get_event_log(game.g);
      }
      let formatted = payload;
      try {
        formatted = JSON.stringify(JSON.parse(payload), null, 2);
      } catch {
        // leave as raw string
      }
      setEventLogText(formatted);
    } catch (err) {
      console.error('fetch event log failed', err);
    }
  }, [game, useWorker]);

  const undoMove = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        await game.call('undo');
      } else {
        game.mod.undo(game.g);
      }
      await syncBoard();
    } catch (err) {
      console.error('undo failed', err);
    }
  }, [game, useWorker, syncBoard]);

  const redoMove = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        await game.call('redo');
      } else {
        game.mod.redo(game.g);
      }
      await syncBoard();
    } catch (err) {
      console.error('redo failed', err);
    }
  }, [game, useWorker, syncBoard]);

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
        <button onClick={undoMove} disabled={!game} data-testid="playground-undo">Undo</button>
        <button onClick={redoMove} disabled={!game} data-testid="playground-redo">Redo</button>
        <button onClick={() => {
          if (showMoves) {
            clearMoves();
          } else {
            fetchMoves();
          }
        }} disabled={!game || loadingMoves}>
          {showMoves ? 'Hide legal moves' : 'Show legal moves'}
        </button>
        {loadingMoves && <span style={{fontSize:12}}> loading…</span>}
        <label>CPU:
          <select
            value={cpuDifficulty}
            onChange={e => setCpuDifficulty(e.target.value as 'off' | 'easy' | 'medium' | 'hard')}
            style={{marginLeft: 4}}
          >
            <option value="off">Off</option>
            <option value="easy">Easy</option>
            <option value="medium">Medium</option>
            <option value="hard">Hard</option>
          </select>
        </label>
        <button
          onClick={requestCpuHint}
          disabled={!game || cpuDifficulty === 'off' || cpuThinking}
        >
          CPU Hint
        </button>
        <button
          onClick={playCpuSuggestion}
          disabled={!game || cpuSuggestion == null || cpuThinking}
        >
          Play as CPU
        </button>
        {cpuThinking && <span style={{fontSize: 12}}> computing…</span>}
        <button onClick={exportSnapshot} disabled={!game}>Save Snapshot</button>
        <button
          onClick={importSnapshot}
          disabled={!game || snapshotText.trim() === ''}
        >
          Load Snapshot
        </button>
        <button onClick={fetchEventLog} disabled={!game}>Show Event Log</button>
      </div>
      <div style={{display:'flex', gap: 16, alignItems:'flex-start'}}>
        <div style={{display: 'grid', gridTemplateColumns: `repeat(${board.width}, 28px)`, gap: 4}}>
          {Array.from({length: use3D ? Math.floor(board.height / depth) : board.height}).map((_, y) => (
            Array.from({length: board.width}).map((__, x) => (
              <div key={`${x}-${y}`}
                   data-testid="playground-board-cell"
                   data-x={x}
                   data-y={use3D ? (y + z * Math.floor(board.height / depth)) : y}
                   onDragOver={(e)=>e.preventDefault()}
                   onDrop={(e)=>onDropCell(x, y, e)}
                   style={{
                     width: 28,
                     height: 28,
                     border: '1px solid #ccc',
                     display:'flex',
                     alignItems:'center',
                     justifyContent:'center',
                     background: highlightCells.has(`${x},${use3D ? (y + z * Math.floor(board.height / depth)) : y}`) ? '#e0f2fe' : '#fff'
                   }}
                   onMouseEnter={() => {
                     if (!showMoves) return;
                     const gy = use3D ? (y + z * Math.floor(board.height / depth)) : y;
                     const idx = legalMoves.findIndex(mv => mv.placements.some(p => p.x === x && p.y === gy));
                     if (idx >= 0) setActiveMoveIndex(idx);
                   }}
               >
                {cellDisplay(x,y).slice(0,1)}
              </div>
            ))
          ))}
        </div>
        <div>
          <div style={{marginBottom: 6, fontSize: 12, opacity: 0.7}}>Rack (drag onto board)</div>
          <div style={{display:'flex', gap: 6}}>
            {rack.map((k, i) => (
              <div key={i}
                   draggable
                   data-testid="playground-rack-tile"
                   data-kind={k}
                   onDragStart={(e)=>onDragStartTile(k, e)}
                   style={{width:28, height:28, border:'1px solid #aaa', display:'flex', alignItems:'center', justifyContent:'center', background:'#f9f9f9', cursor:'grab'}}>
                {k}
              </div>
            ))}
          </div>
          {cpuSuggestion && (
            <div style={{marginTop: 12, fontSize: 12, padding: 8, border: '1px solid var(--ifm-color-emphasis-200)', borderRadius: 4}}>
              <div style={{fontWeight: 600, marginBottom: 4}}>CPU ({cpuSuggestion.difficulty}) suggests:</div>
              <div><strong>{cpuSuggestion.word}</strong> — {cpuSuggestion.total} pts</div>
              <div style={{opacity:0.7}}>Raw {cpuSuggestion.score}, leave {cpuSuggestion.rackLeave}, equity {cpuSuggestion.boardEquity}, endgame {cpuSuggestion.endgamePenalty}</div>
            </div>
          )}
          {!cpuSuggestion && lastCpu && (
            <div style={{marginTop: 12, fontSize: 12, padding: 8, border: '1px solid var(--ifm-color-emphasis-200)', borderRadius: 4}}>
              <div style={{fontWeight: 600, marginBottom: 4}}>Last CPU hint ({lastCpu.difficulty}):</div>
              <div><strong>{lastCpu.word}</strong> — {lastCpu.total} pts</div>
              <div style={{opacity:0.7}}>Raw {lastCpu.score}, leave {lastCpu.rackLeave}, equity {lastCpu.boardEquity}, endgame {lastCpu.endgamePenalty}</div>
            </div>
          )}
        </div>
        {showMoves && (
          <div style={{minWidth: 180, maxWidth: 220, fontSize: 13}}>
            <div style={{fontWeight: 600, marginBottom: 8}}>Legal moves</div>
            {legalMoves.length === 0 && !loadingMoves && (
              <div style={{opacity: 0.7}}>No moves available for the current rack.</div>
            )}
            {legalMoves.map((mv, idx) => (
              <div key={`${mv.word}-${idx}`} style={{marginBottom: 8, paddingBottom: 8, borderBottom: '1px solid var(--ifm-color-emphasis-200)'}}>
                <div style={{fontWeight: 600}}>
                  #{idx + 1} {mv.word} <span style={{opacity:0.7}}>({mv.score} pts)</span>
                </div>
                <div style={{marginTop: 4, display:'flex', gap: 6}}>
                  <button onClick={() => setActiveMoveIndex(idx)} style={{fontSize:12}}>Highlight</button>
                  <button onClick={() => playGeneratedMove(mv)} style={{fontSize:12}}>Play</button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      <div style={{marginTop: 16}}>
        <div style={{fontSize: 12, fontWeight: 600, marginBottom: 4}}>Snapshot JSON</div>
        <textarea
          value={snapshotText}
          onChange={e => setSnapshotText(e.target.value)}
          rows={4}
          style={{width: '100%', fontFamily: 'monospace'}}
          placeholder="Click Save Snapshot to capture the current game state"
        />
      </div>
      {eventLogText && (
        <div style={{marginTop: 12}}>
          <div style={{fontSize: 12, fontWeight: 600, marginBottom: 4}}>Event Log</div>
          <pre style={{maxHeight: 180, overflow: 'auto', background: '#f9fafb', padding: 8, border: '1px solid var(--ifm-color-emphasis-200)'}}>
            {eventLogText}
          </pre>
        </div>
      )}
    </div>
  );
}
