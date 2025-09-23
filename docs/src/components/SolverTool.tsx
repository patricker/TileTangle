import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {classicTilesets, classicBonuses} from './demoUtils';

type SolverMove = {
  word: string;
  score: number;
  placements: { x: number; y: number; kind_id: string }[];
};

type WorkerHandle = {
  call: (action: string, payload?: any) => Promise<any>;
  terminate: () => void;
};

function useWasmWorker(): WorkerHandle {
  const workerRef = useRef<Worker | null>(null);
  const workerUrl = useBaseUrl('wasm/engine/worker.js');

  useEffect(() => () => {
    if (workerRef.current) {
      workerRef.current.terminate();
      workerRef.current = null;
    }
  }, []);

  const call = useCallback((action: string, payload?: any) => {
    if (!workerRef.current) {
      workerRef.current = new Worker(workerUrl, {type: 'module'});
    }
    const worker = workerRef.current;
    return new Promise<any>((resolve, reject) => {
      const id = Math.random().toString(36).slice(2);
      const onMsg = (event: MessageEvent) => {
        if ((event.data as any)?.id === id) {
          worker.removeEventListener('message', onMsg);
          (event.data as any).ok ? resolve(event.data) : reject(new Error((event.data as any).error));
        }
      };
      worker.addEventListener('message', onMsg);
      worker.postMessage({id, action, payload});
    });
  }, [workerUrl]);

  const terminate = useCallback(() => {
    if (workerRef.current) {
      workerRef.current.terminate();
      workerRef.current = null;
    }
  }, []);

  return {call, terminate};
}

export default function SolverTool(): JSX.Element {
  const {call} = useWasmWorker();
  const [rackInput, setRackInput] = useState('AEIRST?');
  const [limitInput, setLimitInput] = useState(10);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [moves, setMoves] = useState<SolverMove[]>([]);

  const dictionaryBytesRef = useRef<Uint8Array | null>(null);
  const dictionaryTextRef = useRef<string | null>(null);

  const baseConfig = useMemo(() => {
    const {tile_kinds, tile_counts} = classicTilesets();
    return {
      tileset: {tile_kinds},
      rack_size: 7,
      board_layout: {width: 15, height: 15},
      ruleset_id: 'cross',
      dictionary_id: 'en',
      rng_seed: 99,
      tile_counts,
      free_word_mode: false,
    } as any;
  }, []);

  const fstUrl = useBaseUrl('dictionaries/TWL06.fst');
  const txtUrl = useBaseUrl('dictionaries/TWL06.txt');

  const ensureDictionary = useCallback(async () => {
    if (dictionaryBytesRef.current || dictionaryTextRef.current) {
      return;
    }
    try {
      const resp = await fetch(fstUrl);
      if (resp.ok) {
        dictionaryBytesRef.current = new Uint8Array(await resp.arrayBuffer());
        return;
      }
    } catch (err) {
      console.warn('Failed to fetch FST dictionary, falling back to text', err);
    }
    const txtResp = await fetch(txtUrl);
    if (txtResp.ok) {
      dictionaryTextRef.current = await txtResp.text();
    }
  }, [fstUrl, txtUrl]);

  const solve = useCallback(async () => {
    const letters = rackInput.replace(/[^A-Za-z\?]/g, '').toUpperCase().split('');
    if (letters.length === 0) {
      setError('Enter rack letters (use ? for blanks)');
      setMoves([]);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await call('new_game', {config: baseConfig, players: 2});
      await call('set_reading_direction', {rtl: false});
      await call('set_stacking', {enabled: false, max_height: 7, forbid_same: true, scoring: 'top'});
      await call('set_free_word_mode', {on: false});
      await ensureDictionary();
      if (dictionaryBytesRef.current) {
        await call('set_dictionary_from_fst_bytes', {bytes: dictionaryBytesRef.current, case_fold: true});
      } else if (dictionaryTextRef.current) {
        await call('set_dictionary_from_text', {text: dictionaryTextRef.current, case_fold: true});
      }
      await call('set_bonuses', classicBonuses());
      const rackTiles = letters.map(ch => (ch === '?' ? 'BL' : ch));
      await call('set_rack', {tiles: rackTiles});
      const resp = await call('generate_moves', {max_len: 15, limit: Math.max(1, Math.min(50, limitInput))});
      const parsed: SolverMove[] = JSON.parse(String((resp as any)?.moves ?? '[]'));
      const topMoves = parsed
        .sort((a, b) => b.score - a.score)
        .slice(0, Math.max(1, Math.min(50, limitInput)));
      setMoves(topMoves);
    } catch (err) {
      console.error('solve failed', err);
      setError(err instanceof Error ? err.message : String(err));
      setMoves([]);
    } finally {
      setLoading(false);
    }
  }, [rackInput, limitInput, call, baseConfig, ensureDictionary]);

  return (
    <div style={{border: '1px solid var(--ifm-color-emphasis-200)', borderRadius: 8, padding: 16, marginBottom: 24}}>
      <div style={{display: 'flex', flexWrap: 'wrap', gap: 12, alignItems: 'center'}}>
        <label style={{fontSize: 14}}>
          Rack
          <input
            type="text"
            value={rackInput}
            onChange={e => setRackInput(e.target.value)}
            placeholder="AEIRST?"
            style={{marginLeft: 8, padding: '4px 8px', fontFamily: 'monospace'}}
            maxLength={12}
            data-testid="solver-rack-input"
          />
        </label>
        <label style={{fontSize: 14}}>
          Limit
          <input
            type="number"
            min={1}
            max={30}
            value={limitInput}
            onChange={e => setLimitInput(Math.max(1, Math.min(30, Number(e.target.value) || 1)))}
            style={{marginLeft: 8, width: 64}}
          />
        </label>
        <button
          type="button"
          onClick={solve}
          disabled={loading}
          className="button button--primary"
          data-testid="solver-submit"
        >
          {loading ? 'Solving…' : 'Solve' }
        </button>
        <span style={{fontSize: 12, opacity: 0.7}}>Use '?' for blanks. Results assume an empty board with classic bonuses.</span>
      </div>
      {error && <div style={{marginTop: 12, color: 'var(--ifm-color-danger)'}}>{error}</div>}
      {!error && moves.length > 0 && (
        <div style={{marginTop: 16}}>
          <div style={{fontWeight: 600, marginBottom: 8}}>Top moves</div>
          <ol style={{paddingLeft: 20, fontSize: 14}}>
            {moves.map((mv, idx) => (
              <li key={`${mv.word}-${idx}`} style={{marginBottom: 6}}>
                <span style={{fontWeight: 600}}>{mv.word}</span>
                {` — ${mv.score} pts`}
                <div style={{fontSize: 12, opacity: 0.7}}>Placements: {mv.placements.map(p => `(${p.x},${p.y}) ${p.kind_id}`).join(', ')}</div>
              </li>
            ))}
          </ol>
        </div>
      )}
      {!error && !moves.length && !loading && (
        <div style={{marginTop: 16, fontSize: 13, opacity: 0.7}}>
          Enter a rack and click Solve to view candidate plays.
        </div>
      )}
      <div style={{marginTop: 16, fontSize: 13}}>
        <strong>Python equivalent:</strong> run <code>python examples/python/top_moves.py</code> and edit the <code>rack_letters</code>
        variable to mirror the rack above.
      </div>
    </div>
  );
}
