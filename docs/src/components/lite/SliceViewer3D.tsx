import React, {useMemo, useState} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import PlaygroundBoard from '../playground/PlaygroundBoard';
import type {BoardJson} from '../playground/types';
import styles from '../PlaygroundLayout.module.css';
import {getPlaygroundPalette} from '../playground/theme';

export type SliceViewer3DProps = {
  width: number;
  height: number;
  depth: number;
  tileSize?: number;
};

export default function SliceViewer3D({width, height, depth, tileSize = 26}: SliceViewer3DProps): JSX.Element {
  const {colorMode} = useColorMode();
  const palette = useMemo(() => getPlaygroundPalette(colorMode as 'light' | 'dark'), [colorMode]);
  const [layer, setLayer] = useState(0);

  const board: BoardJson = useMemo(() => ({
    width,
    height,
    rows: Array.from({length: height}, () => Array.from({length: width}, () => '')),
  }), [width, height]);

  const isCellActive = (x: number, y: number) => x >= 0 && x < width && y >= 0 && y < height * depth;
  const cellDisplay = (x: number, viewY: number) => board.rows[viewY]?.[x] ?? '';

  return (
    <div>
      <div className={styles.boardWrapper}>
        <PlaygroundBoard
          board={board}
          layerHeight={height}
          currentLayer={layer}
          effectiveDepth={depth}
          use3D
          cellSize={tileSize}
          cellGap={4}
          tileFontSize={12}
          tileScoreFontSize={10}
          highlightCells={new Set()}
          isCellActive={isCellActive}
          cellDisplay={cellDisplay}
          getTileMeta={() => undefined}
          onDropCell={() => {}}
          showMoves={false}
          onHoverForMoves={() => {}}
          palette={{
            boardCellBorder: palette.boardCellBorder,
            boardCellHighlight: palette.boardCellHighlight,
            boardCellBg: palette.boardCellBg,
            boardHexBorder: palette.boardHexBorder,
          }}
          tileShape="square"
          readonly
          renderOverlay={() => null}
        />
      </div>
      <label className={styles.layerSlider}>
        <span>Viewing layer {layer + 1} / {depth}</span>
        <input
          type="range"
          min={0}
          max={Math.max(0, depth - 1)}
          value={layer}
          onChange={e => setLayer(Number(e.target.value))}
        />
      </label>
    </div>
  );
}

