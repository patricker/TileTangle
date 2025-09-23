import React, {useMemo} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import PlaygroundBoard from '../playground/PlaygroundBoard';
import {getPlaygroundPalette} from '../playground/theme';
import type {BoardJson} from '../playground/types';

function makeBoard(): BoardJson {
  const width = 9;
  const height = 9;
  const rows: string[][] = Array.from({length: height}, () => Array.from({length: width}, () => ''));
  // Horizontal word: PLAY (x:2..5, y:4)
  rows[4][2] = 'P';
  rows[4][3] = 'L';
  rows[4][4] = 'A';
  rows[4][5] = 'Y';
  // Vertical word: AI (x:4, y:2..3) crossing at A (4,4)
  rows[2][4] = 'A';
  rows[3][4] = 'I';
  return {width, height, rows};
}

const letterScores = new Map<string, number>([
  ['A', 1], ['I', 1], ['L', 1], ['P', 3], ['Y', 4],
]);

const anchorKeys = new Set<string>([
  // Around word ends and junctions
  '1,4', '6,4', // left and right of PLAY
  '4,1', '4,6', // above/below the vertical stem
  '3,5', '5,3', // diagonals near the junction
]);

export default function LivePlaygroundPreview(): JSX.Element {
  const {colorMode} = useColorMode();
  const palette = useMemo(() => getPlaygroundPalette(colorMode as 'light' | 'dark'), [colorMode]);
  const board = useMemo(makeBoard, []);

  const cellSize = 22;
  const tileFont = 12;
  const tileScoreFont = 10;

  return (
    <div aria-label="Live playground preview showing anchors" style={{display: 'inline-block'}}>
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
        highlightCells={new Set(['2,4','3,4','4,4','5,4','4,2','4,3'])}
        isCellActive={() => true}
        cellDisplay={(x, y) => board.rows[y][x]}
        getTileMeta={id => id ? {symbol: id, score: letterScores.get(id) ?? 0} : undefined}
        onDropCell={() => undefined}
        showMoves={false}
        onHoverForMoves={() => undefined}
        palette={palette}
        readonly
        renderOverlay={({x, globalY}) => {
          const key = `${x},${globalY}`;
          if (anchorKeys.has(key) && !board.rows[globalY][x]) {
            return (
              <span style={{
                position: 'absolute',
                inset: 0,
                display: 'grid',
                placeItems: 'center',
                color: palette.boardCellBorder,
                fontSize: 12,
                opacity: 0.9,
              }}>+</span>
            );
          }
          return null;
        }}
      />
    </div>
  );
}

