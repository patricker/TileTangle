import React, {useMemo, useState} from 'react';
import MiniBoard from './MiniBoard';
import type {BoardJson} from '../playground/types';
import {Panel, ToggleField, SegmentedControl} from '../playground/ui';
import styles from '../PlaygroundLayout.module.css';

type Adj = 'orthogonal' | 'diagonal' | 'hex';

export default function RulesDemoLite(): JSX.Element {
  const [freeWord, setFreeWord] = useState(true);
  const [stackOn, setStackOn] = useState(false);
  const [forbidSame, setForbidSame] = useState(true);
  const [adj, setAdj] = useState<Adj>('orthogonal');

  const width = 9, height = 9;
  const board: BoardJson = useMemo(() => ({
    width,
    height,
    rows: Array.from({length: height}, () => Array.from({length: width}, () => '')),
  }), []);

  const midX = Math.floor(width / 2);
  const midY = Math.floor(height / 2);

  return (
    <Panel title="Rules demo" subtitle="Legal anchors and toggles (visual)." density="compact">
      <div className={styles.stageSplit}>
        <div>
          <div className={styles.boardWrapper}>
            <MiniBoard
              board={board}
              tileMeta={() => undefined}
              overlay={({x, y}) => {
                // Center cell is the required anchor for the first move.
                if (x === midX && y === midY) {
                  return <div className={styles.hintBadge}>★ center</div>;
                }
                // Show example anchor neighbors around center depending on adjacency mode
                const isOrth = (x === midX && Math.abs(y - midY) === 1) || (y === midY && Math.abs(x - midX) === 1);
                const isDiag = Math.abs(x - midX) === 1 && Math.abs(y - midY) === 1;
                const isHex = isOrth || isDiag; // visually indicate 6 neighbors around center
                const show = adj === 'orthogonal' ? isOrth : adj === 'diagonal' ? (isOrth || isDiag) : isHex;
                if (show) {
                  return <div className={styles.hintBadge}>anchor</div>;
                }
                // Illustrate stacking toggle at an example cell
                if (stackOn && x === midX && y === midY + 2) {
                  return <div className={styles.hintBadge}>stack on</div>;
                }
                return null;
              }}
            />
          </div>
        </div>
        <div>
          <div className={styles.modeGroup}>
            <div className={styles.modeLabel}>Adjacency</div>
            <SegmentedControl
              name="Adjacency"
              value={adj}
              options={[
                {value: 'orthogonal', label: 'Orthogonal'},
                {value: 'diagonal', label: '8‑way'},
                {value: 'hex', label: 'Hex'},
              ]}
              onChange={v => setAdj(v as Adj)}
            />
          </div>
          <ToggleField label="Free‑word mode (skip dictionary)" checked={freeWord} onChange={setFreeWord} />
          <ToggleField label="Enable stacking" checked={stackOn} onChange={setStackOn} />
          <ToggleField label="Forbid identical overlays" checked={forbidSame} onChange={setForbidSame} />
          <div className={styles.helperText} style={{marginTop: 8}}>
            {adj === 'orthogonal' && 'Legal directions: N/E/S/W. First move must cover center.'}
            {adj === 'diagonal' && 'Legal directions: N/E/S/W plus diagonals. First move must cover center.'}
            {adj === 'hex' && 'Hex-like adjacency around each cell (three axes). First move must cover center.'}
          </div>
          <div className={styles.helperText}>
            {freeWord ? 'Dictionary checks are disabled.' : 'Dictionary checks are enabled.'}
            {' '}
            {stackOn ? `Stacking is ON (${forbidSame ? 'no identical overlays' : 'overlays allowed'}).` : 'Stacking is OFF.'}
          </div>
        </div>
      </div>
    </Panel>
  );
}

