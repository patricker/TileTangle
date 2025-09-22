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

  // Build a simple 3D sample with letters on different layers to demonstrate the slider.
  const cells3d = useMemo(() => {
    const layers: string[][][] = [];
    for (let z = 0; z < depth; z++) {
      const rows: string[][] = Array.from({length: height}, () => Array.from({length: width}, () => ''));
      layers.push(rows);
    }
    const midX = Math.floor(width / 2);
    const midY = Math.floor(height / 2);
    // Layer 1 pattern (z=0): horizontal trio
    if (depth >= 1) {
      const y = Math.max(0, midY - 1);
      const xs = [midX - 1, midX, midX + 1].filter(x => x >= 0 && x < width);
      const letters = ['A', 'B', 'C'];
      xs.forEach((x, i) => (layers[0][y][x] = letters[i] ?? ''));
    }
    // Layer 2 pattern (z=1): centered trio
    if (depth >= 2) {
      const y = midY;
      const xs = [midX - 1, midX, midX + 1].filter(x => x >= 0 && x < width);
      const letters = ['D', 'E', 'F'];
      xs.forEach((x, i) => (layers[1][y][x] = letters[i] ?? ''));
    }
    // Layer 3 pattern (z=2): lower trio
    if (depth >= 3) {
      const y = Math.min(height - 1, midY + 1);
      const xs = [midX - 1, midX, midX + 1].filter(x => x >= 0 && x < width);
      const letters = ['G', 'H', 'I'];
      xs.forEach((x, i) => (layers[2][y][x] = letters[i] ?? ''));
    }
    return layers;
  }, [depth, height, width]);

  const board: BoardJson = useMemo(() => ({
    width,
    height,
    rows: Array.from({length: height}, () => Array.from({length: width}, () => '')),
  }), [width, height]);

  const isCellActive = (x: number, y: number) => x >= 0 && x < width && y >= 0 && y < height * depth;
  const cellDisplay = (x: number, viewY: number) => cells3d[layer]?.[viewY]?.[x] ?? '';

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
      <div className={styles.helperText} style={{marginTop: 6}}>
        Letters vary by layer to demonstrate the slider.
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
