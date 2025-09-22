import React, {useCallback, useMemo, useRef, useState} from 'react';
import {classicBonuses, classicTilesets} from '../demoUtils';
import {MoveList, type MoveListItem, Panel, ButtonRow} from '../playground/ui';
import MiniBoard from './MiniBoard';
import type {BoardJson} from '../playground/types';
import styles from '../PlaygroundLayout.module.css';
import {useWorkerMessenger} from '../playground/useWorkerMessenger';

type SolverMove = {
  word: string;
  score: number;
  placements: { x: number; y: number; kind_id: string }[];
};

export type MoveListLiteProps = {
  rack?: string[]; // default AEIRST?
  limit?: number; // default 10
};

export default function MoveListLite({rack = ['A','E','I','R','S','T','BL'], limit = 10}: MoveListLiteProps): JSX.Element {
  const {ensureWorker, callWorker, terminateWorker} = useWorkerMessenger();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [items, setItems] = useState<MoveListItem[]>([]);
  const [moves, setMoves] = useState<SolverMove[]>([]);
  const initializedRef = useRef(false);
  const [activeIdx, setActiveIdx] = useState<number | null>(null);

  const classic = useMemo(() => classicTilesets(), []);
  const board: BoardJson = useMemo(() => ({
    width: 15,
    height: 15,
    rows: Array.from({length: 15}, () => Array.from({length: 15}, () => '')),
  }), []);
  const tileMeta = useMemo(() => {
    const map = new Map<string, {symbol: string; score: number}>();
    for (const k of classic.tile_kinds) map.set(k.id, {symbol: k.symbol || k.id, score: k.score});
    return (id: string) => map.get(id);
  }, [classic.tile_kinds]);

  const setup = useCallback(async () => {
    if (initializedRef.current) return;
    const worker = ensureWorker();
    if (!worker) throw new Error('Worker unavailable');
    const cfg = {
      tileset: {tile_kinds: classic.tile_kinds},
      rack_size: 7,
      board_layout: {width: 15, height: 15},
      ruleset_id: 'cross',
      dictionary_id: 'en',
      rng_seed: 7,
      tile_counts: classic.tile_counts,
      free_word_mode: true,
    } as any;
    await callWorker('new_game', {config: cfg, players: 2});
    await callWorker('set_reading_direction', {rtl: false});
    await callWorker('set_stacking', {enabled: false, max_height: 7, forbid_same: true, scoring: 'top'});
    await callWorker('set_bonuses', classicBonuses());
    initializedRef.current = true;
  }, [callWorker, ensureWorker]);

  const handleShowMoves = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await setup();
      const rackTiles = rack.map(ch => (ch === '?' ? 'BL' : ch));
      await callWorker('set_rack', {tiles: rackTiles});
      const resp = await callWorker('generate_moves', {max_len: 15, limit: Math.max(1, Math.min(50, limit))});
      const parsed: SolverMove[] = JSON.parse(String((resp as any)?.moves ?? '[]'));
      const trimmed = parsed.slice(0, limit);
      const list: MoveListItem[] = trimmed.map((mv, idx) => ({
        key: `${mv.word}-${idx}`,
        index: idx,
        word: mv.word,
        score: mv.score,
        onHover: () => setActiveIdx(idx),
        onLeave: () => setActiveIdx(prev => (prev === idx ? null : prev)),
      }));
      setItems(list);
      setMoves(trimmed);
      setActiveIdx(null);
    } catch (err) {
      console.error('MoveListLite failed', err);
      setError(err instanceof Error ? err.message : String(err));
      setItems([]);
      setMoves([]);
    } finally {
      setLoading(false);
    }
  }, [callWorker, limit, rack, setup]);

  const reset = useCallback(() => {
    setItems([]);
    setError(null);
  }, []);

  const buttons = useMemo(() => ([
    { key: 'show', label: loading ? 'Loading…' : 'Show moves', onClick: handleShowMoves, disabled: loading },
    { key: 'reset', label: 'Reset', onClick: reset, disabled: loading && items.length === 0 },
  ]), [handleShowMoves, items.length, loading, reset]);

  return (
    <Panel title="Top moves (empty board)" subtitle="Free-word mode, classic bonuses." density="compact">
      <ButtonRow buttons={buttons} />
      {error && <div style={{color: 'var(--ifm-color-danger)'}}>{error}</div>}
      <div className={styles.stageSplit}>
        <div>
          <MoveList items={items} emptyMessage="Click Show moves to fetch candidates." />
        </div>
        <div>
          <div className={styles.helperText} style={{marginBottom: 6}}>Hover a move to preview placements:</div>
          <div className={styles.boardWrapper}>
            <MiniBoard
              board={board}
              tileMeta={tileMeta}
              overlay={({x, y}) => {
                if (activeIdx == null) return null;
                const mv = moves[activeIdx];
                if (!mv) return null;
                const hit = mv.placements.find(p => p.x === x && p.y === y);
                if (!hit) return null;
                return <div className={styles.hintBadge}>{(hit.kind_id || '').slice(0, 2)}</div>;
              }}
            />
          </div>
        </div>
      </div>
    </Panel>
  );
}
