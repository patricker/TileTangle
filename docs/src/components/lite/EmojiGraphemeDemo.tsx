import React, {useMemo, useState} from 'react';
import MiniBoard from './MiniBoard';
import type {BoardJson} from '../playground/types';
import styles from '../PlaygroundLayout.module.css';

const kinds = [
  { id: 'ASTRONAUT', symbol: '👩‍🚀', score: 6 },
  { id: 'SATELLITE', symbol: '🛰️', score: 5 },
  { id: 'STAR', symbol: '⭐', score: 4 },
  { id: 'GRIN', symbol: '😀', score: 1 },
];

export default function EmojiGraphemeDemo(): JSX.Element {
  const [showGraphemes, setShowGraphemes] = useState(false);
  const board: BoardJson = useMemo(() => ({
    width: 7,
    height: 7,
    rows: [
      ['', '', '', '', '', '', ''],
      ['', '', '', 'STAR', '', '', ''],
      ['', '', 'GRIN', '', 'STAR', '', ''],
      ['', 'SATELLITE', '', 'ASTRONAUT', '', 'GRIN', ''],
      ['', '', 'STAR', '', 'GRIN', '', ''],
      ['', '', '', 'STAR', '', '', ''],
      ['', '', '', '', '', '', ''],
    ],
  }), []);

  const tileMeta = useMemo(() => {
    const map = new Map<string, {symbol: string; score: number}>();
    kinds.forEach(k => map.set(k.id, {symbol: k.symbol, score: k.score}));
    return (id: string) => map.get(id);
  }, []);

  return (
    <div>
      <div style={{display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8}}>
        <label className={styles.toggleRow}>
          <input type="checkbox" checked={showGraphemes} onChange={e => setShowGraphemes(e.target.checked)} />
          <span>Show grapheme info</span>
        </label>
        <span className={styles.helperText}>Emoji tiles render as single graphemes even when composed via ZWJ/variation selectors.</span>
      </div>
      <div className={styles.boardWrapper}>
        <MiniBoard
          board={board}
          tileMeta={tileMeta}
          overlay={({x, y, placement}) => {
            if (!placement || !showGraphemes) return null;
            const sym = (kinds.find(k => k.id === placement)?.symbol) || '';
            const seg = (Intl as any)?.Segmenter ? new (Intl as any).Segmenter(undefined, {granularity: 'grapheme'}) : null;
            let parts = 1;
            if (seg) {
              parts = Array.from(seg.segment(sym)).length;
            }
            const label = parts > 1 ? `${parts} graphemes` : '1 grapheme';
            return <div className={styles.hintBadge}>{label}</div>;
          }}
        />
      </div>
    </div>
  );
}

