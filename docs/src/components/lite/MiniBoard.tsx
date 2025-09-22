import React, {useMemo} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import PlaygroundBoard from '../playground/PlaygroundBoard';
import type {BoardJson} from '../playground/types';
import {getPlaygroundPalette} from '../playground/theme';

export type MiniBoardProps = {
  board: BoardJson;
  tileMeta: (id: string) => {symbol: string; score: number} | undefined;
  cellSize?: number;
  cellGap?: number;
  tileFontSize?: number;
  tileScoreFontSize?: number;
  tileShape?: 'square' | 'hex';
  use3D?: boolean;
  currentLayer?: number;
  depth?: number;
  overlay?: (args: { x: number; y: number; placement: string }) => React.ReactNode;
  highlight?: Set<string>;
};

export default function MiniBoard({
  board,
  tileMeta,
  cellSize = 28,
  cellGap = 4,
  tileFontSize = 14,
  tileScoreFontSize = 11,
  tileShape = 'square',
  use3D = false,
  currentLayer = 0,
  depth = 1,
  overlay,
  highlight,
}: MiniBoardProps): JSX.Element {
  const {colorMode} = useColorMode();
  const palette = useMemo(() => getPlaygroundPalette(colorMode as 'light' | 'dark'), [colorMode]);

  const layerHeight = useMemo(() => (use3D ? board.height : board.height), [board, use3D]);

  const cellDisplay = (x: number, viewY: number): string => {
    const row = board.rows[viewY];
    return row && row[x] ? row[x] : '';
  };

  const isCellActive = (x: number, y: number): boolean => {
    return x >= 0 && x < board.width && y >= 0 && y < board.height * (use3D ? 1 : 1);
  };

  const highlightCells = highlight ?? new Set<string>();

  return (
    <PlaygroundBoard
      board={board}
      layerHeight={layerHeight}
      currentLayer={currentLayer}
      effectiveDepth={use3D ? Math.max(1, depth) : 1}
      use3D={use3D}
      cellSize={cellSize}
      cellGap={cellGap}
      tileFontSize={tileFontSize}
      tileScoreFontSize={tileScoreFontSize}
      highlightCells={highlightCells}
      isCellActive={isCellActive}
      cellDisplay={cellDisplay}
      getTileMeta={tileMeta}
      onDropCell={() => { /* no-op in MiniBoard */ }}
      showMoves={false}
      onHoverForMoves={() => { /* no-op */ }}
      palette={{
        boardCellBorder: palette.boardCellBorder,
        boardCellHighlight: palette.boardCellHighlight,
        boardCellBg: palette.boardCellBg,
        boardHexBorder: palette.boardHexBorder,
      }}
      tileShape={tileShape}
      readonly
      onCellClick={undefined}
      renderOverlay={args => (overlay ? overlay({x: args.x, y: args.globalY, placement: args.placement}) : null)}
    />
  );
}
