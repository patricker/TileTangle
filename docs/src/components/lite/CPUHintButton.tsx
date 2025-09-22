import React, {useCallback, useMemo, useState} from 'react';
import {classicBonuses, classicTilesets} from '../demoUtils';
import {CpuHintSummary, Panel, ButtonRow} from '../playground/ui';
import {useWorkerMessenger} from '../playground/useWorkerMessenger';

type SolverMove = {
  word: string;
  score: number;
  placements: { x: number; y: number; kind_id: string }[];
};

export type CPUHintButtonProps = {
  difficulty?: 'easy' | 'medium' | 'hard'; // visual only; we approximate via generate_moves
  rack?: string[];
  label?: string;
};

export default function CPUHintButton({difficulty = 'medium', rack = ['T','I','D','E','R','S','A'], label = 'Suggest move'}: CPUHintButtonProps): JSX.Element {
  const {ensureWorker, callWorker} = useWorkerMessenger();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [move, setMove] = useState<SolverMove | null>(null);

  const setup = useCallback(async () => {
    const worker = ensureWorker();
    if (!worker) throw new Error('Worker unavailable');
    const classic = classicTilesets();
    const cfg = {
      tileset: {tile_kinds: classic.tile_kinds},
      rack_size: 7,
      board_layout: {width: 15, height: 15},
      ruleset_id: 'cross',
      dictionary_id: 'en',
      rng_seed: 42,
      tile_counts: classic.tile_counts,
      free_word_mode: true, // keep light-weight; no dictionary required
    } as any;
    await callWorker('new_game', {config: cfg, players: 2});
    await callWorker('set_stacking', {enabled: false, max_height: 7, forbid_same: true, scoring: 'top'});
    await callWorker('set_bonuses', classicBonuses());
  }, [callWorker, ensureWorker]);

  const handleSuggest = useCallback(async () => {
    setLoading(true);
    setError(null);
    setMove(null);
    try {
      await setup();
      const tiles = rack.map(ch => (ch === '?' ? 'BL' : ch));
      await callWorker('set_rack', {tiles});
      const resp = await callWorker('generate_moves', {max_len: 15, limit: 30});
      const parsed: SolverMove[] = JSON.parse(String((resp as any)?.moves ?? '[]'));
      const top = parsed.sort((a, b) => b.score - a.score)[0] || null;
      setMove(top);
    } catch (err) {
      console.error('CPUHintButton failed', err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [callWorker, rack, setup]);

  const buttons = useMemo(() => ([{key: 'hint', label: loading ? 'Computing…' : label, onClick: handleSuggest, disabled: loading}]), [handleSuggest, label, loading]);

  return (
    <Panel title="CPU Hint (lite)" subtitle="Greedy suggestion on an empty board." density="compact">
      <ButtonRow buttons={buttons} />
      {error && <div style={{color: 'var(--ifm-color-danger)'}}>{error}</div>}
      {move && (
        <CpuHintSummary
          title={<>CPU ({difficulty}) suggests</>}
          word={move.word || '—'}
          total={<>{move.score} pts</>}
          meta={<>Placements: {move.placements.map(p => `(${p.x},${p.y})`).join(', ') || '—'}</>}
        />
      )}
      {!move && !error && !loading && (
        <div className="admonition admonition-tip" style={{marginTop: 8}}>
          <div className="admonition-content">Click to compute a single suggestion; open the full Playground for difficulty/engine options.</div>
        </div>
      )}
    </Panel>
  );
}

