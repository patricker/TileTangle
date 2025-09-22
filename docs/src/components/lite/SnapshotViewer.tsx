import React, {useCallback, useMemo, useState} from 'react';
import styles from '../PlaygroundLayout.module.css';
import {Panel, ButtonRow} from '../playground/ui';
import {useWorkerMessenger} from '../playground/useWorkerMessenger';

export default function SnapshotViewer(): JSX.Element {
  const [text, setText] = useState('');
  const [log, setLog] = useState<any[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const {ensureWorker, callWorker, terminateWorker} = useWorkerMessenger();
  const [init, setInit] = useState(false);

  const loadSnapshot = useCallback(async () => {
    setLoading(true);
    setError(null);
    setLog(null);
    try {
      const worker = ensureWorker();
      if (!worker) throw new Error('Worker unavailable');
      if (!init) {
        // Create a placeholder game instance; snapshot will overwrite state.
        const cfg = { tileset: {tile_kinds: []}, rack_size: 7, board_layout: {width: 9, height: 9}, ruleset_id: 'cross', dictionary_id: 'en', rng_seed: 1, tile_counts: {}, free_word_mode: true } as any;
        await callWorker('new_game', {config: cfg, players: 2}).catch(() => {});
        setInit(true);
      }
      await callWorker('load_snapshot_json', {json: text});
      const resp = await callWorker('event_log');
      const events = JSON.parse(String((resp as any)?.log ?? '[]'));
      setLog(events);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setLog(null);
    } finally {
      setLoading(false);
    }
  }, [callWorker, ensureWorker, text]);

  const buttons = useMemo(() => ([
    {key: 'load', label: loading ? 'Loading…' : 'Load & Show Log', onClick: loadSnapshot, disabled: loading || !text.trim()},
  ]), [loadSnapshot, loading, text]);

  return (
    <Panel title="Snapshot & Log" subtitle="Paste JSON snapshot to view the event stream." density="compact">
      <textarea
        className={styles.snapshotArea}
        rows={6}
        placeholder="Paste snapshot JSON here"
        value={text}
        onChange={e => setText(e.target.value)}
      />
      <ButtonRow buttons={buttons} />
      {error && <div style={{color: 'var(--ifm-color-danger)'}}>{error}</div>}
      {Array.isArray(log) && (
        <div className={styles.logViewer}>
          {log.length === 0 ? (
            <div>No events recorded.</div>
          ) : (
            log.map((e, i) => (
              <div key={i}>
                <strong>#{i + 1}</strong> — turn {String(e.turn ?? '?')}, player {String(e.player ?? '?')}, {String(e.type ?? 'event')}
                {typeof e.score === 'number' ? ` — ${e.score} pts` : ''}
              </div>
            ))
          )}
        </div>
      )}
    </Panel>
  );
}
