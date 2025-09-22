import React, {useMemo} from 'react';
import MiniBoard from './MiniBoard';
import type {BoardJson} from '../playground/types';
import {RackRow, type RackRowTile, Panel} from '../playground/ui';
import styles from '../PlaygroundLayout.module.css';

export default function TilesDemoLite(): JSX.Element {
  const tileKinds = useMemo(() => ([
    {id: 'A', symbol: 'A', score: 1},
    {id: 'TH', symbol: 'TH', score: 4},
    {id: 'GRIN', symbol: '😀', score: 1},
    {id: 'BL', symbol: '?', score: 0},
  ]), []);

  const tileMeta = useMemo(() => {
    const map = new Map<string, {symbol: string; score: number}>();
    tileKinds.forEach(k => map.set(k.id, {symbol: k.symbol, score: k.score}));
    return (id: string) => map.get(id);
  }, [tileKinds]);

  const board: BoardJson = useMemo(() => {
    const width = 9, height = 9;
    const rows: string[][] = Array.from({length: height}, () => Array.from({length: width}, () => ''));
    const midX = Math.floor(width / 2);
    const midY = Math.floor(height / 2);
    rows[midY][midX - 1] = 'GRIN';
    rows[midY][midX] = 'TH';
    rows[midY][midX + 1] = 'A';
    rows[midY + 1][midX] = 'A'; // stacked cell (visualised via overlay)
    return {width, height, rows};
  }, []);

  const rackTiles: RackRowTile[] = useMemo(() => ([
    {key: 'A', symbol: 'A', score: 1, size: 36, fontSize: 18, scoreFontSize: 12},
    {key: 'TH', symbol: 'TH', score: 4, size: 36, fontSize: 18, scoreFontSize: 12},
    {key: 'GRIN', symbol: '😀', score: 1, size: 36, fontSize: 18, scoreFontSize: 12},
    {key: 'BL', symbol: '?', score: 0, size: 36, fontSize: 18, scoreFontSize: 12},
  ]), []);

  return (
    <Panel title="Tiles demo" subtitle="Emoji, multi‑character, and stacked tiles (visual)." density="compact">
      <div className={styles.stageSplit}>
        <div>
          <div className={styles.helperText} style={{marginBottom: 6}}>Center shows multi‑character "TH"; below it is a stacked cell.</div>
          <div className={styles.boardWrapper}>
            <MiniBoard
              board={board}
              tileMeta={tileMeta}
              overlay={({x, y}) => {
                // Visual badge for the stacked cell just below center
                const isStack = x === Math.floor(board.width / 2) && y === Math.floor(board.height / 2) + 1;
                if (isStack) return <div className={styles.hintBadge}>stack ×3</div>;
                return null;
              }}
            />
          </div>
        </div>
        <div>
          <div className={styles.helperText} style={{marginBottom: 6}}>Sample tiles</div>
          <RackRow tiles={rackTiles} />
          <div className={styles.helperText} style={{marginTop: 6}}>Use blanks to mark a chosen letter at commit time.</div>
        </div>
      </div>
    </Panel>
  );
}

