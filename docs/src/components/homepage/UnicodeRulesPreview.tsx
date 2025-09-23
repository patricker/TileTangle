import React, {useMemo} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import PlaygroundBoard from '../playground/PlaygroundBoard';
import {getPlaygroundPalette} from '../playground/theme';
import type {BoardJson} from '../playground/types';

function makeUnicodeBoard(): BoardJson {
  const width = 7;
  const height = 7;
  const rows: string[][] = Array.from({length: height}, () => Array.from({length: width}, () => ''));
  // Emoji row (left-to-right)
  rows[2][1] = '😀';
  rows[2][2] = '🍎';
  rows[2][3] = '🚀';
  // Multi-grapheme vertical tile crossing: "CH"
  rows[1][3] = 'CH';
  rows[3][3] = 'I';
  // A stacked cell (we'll show overlay at 5,4)
  rows[4][4] = 'E';
  return {width, height, rows};
}

const scores = new Map<string, number>([
  ['😀', 0], ['🍎', 0], ['🚀', 0], ['CH', 4], ['I', 1], ['E', 1],
]);

export default function UnicodeRulesPreview(): JSX.Element {
  const {colorMode} = useColorMode();
  const palette = useMemo(() => getPlaygroundPalette(colorMode as 'light' | 'dark'), [colorMode]);
  const board = useMemo(makeUnicodeBoard, []);

  const cellSize = 24;
  const tileFont = 13;
  const tileScoreFont = 10;

  const wordBonusAt = new Set(['1,1']);
  const letterBonusAt = new Set(['5,1']);
  const stackedAt = '4,4';

  return (
    <div aria-label="Unicode and rules preview" style={{display: 'inline-block'}}>
      <PlaygroundBoard
        board={board}
        layerHeight={board.height}
        currentLayer={0}
        effectiveDepth={1}
        use3D={false}
        cellSize={cellSize}
        cellGap={2}
        tileFontSize={tileFont}
        tileScoreFontSize={tileScoreFont}
        highlightCells={new Set(['1,2','2,2','3,2','3,1','3,3'])}
        isCellActive={() => true}
        cellDisplay={(x, y) => board.rows[y][x]}
        getTileMeta={id => id ? {symbol: id, score: scores.get(id) ?? 0} : undefined}
        onDropCell={() => undefined}
        showMoves={false}
        onHoverForMoves={() => undefined}
        palette={palette}
        readonly
        renderOverlay={({x, globalY}) => {
          const key = `${x},${globalY}`;
          // Word multiplier
          if (wordBonusAt.has(key) && !board.rows[globalY][x]) {
            return (
              <span style={{
                position: 'absolute', inset: 0, display: 'grid', placeItems: 'center',
                color: '#b91c1c', fontWeight: 700, fontSize: 11
              }}>2W</span>
            );
          }
          // Letter multiplier
          if (letterBonusAt.has(key) && !board.rows[globalY][x]) {
            return (
              <span style={{
                position: 'absolute', inset: 0, display: 'grid', placeItems: 'center',
                color: '#1d4ed8', fontWeight: 700, fontSize: 11
              }}>2L</span>
            );
          }
          // Stacking indicator
          if (key === stackedAt) {
            return (
              <span style={{
                position: 'absolute', top: 2, left: 2, fontSize: 10, fontWeight: 700,
                padding: '1px 4px', borderRadius: 6, background: 'rgba(15,23,42,0.7)', color: 'white'
              }}>2×</span>
            );
          }
          return null;
        }}
      />
    </div>
  );
}

