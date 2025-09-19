import React from 'react';
import styles from '../PlaygroundLayout.module.css';
import type {BoardJson} from './types';

type BoardPalette = {
  boardCellBorder: string;
  boardCellHighlight: string;
  boardCellBg: string;
};

type PlaygroundBoardProps = {
  board: BoardJson;
  layerHeight: number;
  currentLayer: number;
  effectiveDepth: number;
  use3D: boolean;
  cellSize: number;
  cellGap: number;
  tileFontSize: number;
  tileScoreFontSize: number;
  highlightCells: Set<string>;
  isCellActive: (x: number, y: number) => boolean;
  cellDisplay: (x: number, viewY: number) => string;
  getTileMeta: (id: string) => {symbol: string; score: number} | undefined;
  onDropCell: (x: number, viewY: number, ev: React.DragEvent<HTMLDivElement>) => void;
  showMoves: boolean;
  onHoverForMoves: (x: number, y: number) => void;
  palette: BoardPalette;
  renderOverlay?: (args: {
    x: number;
    viewY: number;
    globalY: number;
    placement: string;
    meta?: {symbol: string; score: number};
    highlighted: boolean;
  }) => React.ReactNode;
};

const PlaygroundBoard: React.FC<PlaygroundBoardProps> = ({
  board,
  layerHeight,
  currentLayer,
  effectiveDepth,
  use3D,
  cellSize,
  cellGap,
  tileFontSize,
  tileScoreFontSize,
  highlightCells,
  isCellActive,
  cellDisplay,
  getTileMeta,
  onDropCell,
  showMoves,
  onHoverForMoves,
  palette,
  renderOverlay,
}) => {
  const cells: React.ReactNode[] = [];

  for (let viewRow = 0; viewRow < layerHeight; viewRow++) {
    const globalRow = use3D ? viewRow + currentLayer * layerHeight : viewRow;
    for (let x = 0; x < board.width; x++) {
      const activeCell = use3D || isCellActive(x, globalRow);
      const placement = cellDisplay(x, viewRow);
      const meta = placement ? getTileMeta(placement) : undefined;
      const symbol = meta?.symbol ?? placement?.slice(0, 2) ?? '';
      const score = meta?.score;
      const highlighted = highlightCells.has(`${x},${globalRow}`);
      const overlay = renderOverlay
        ? renderOverlay({x, viewY, globalY: globalRow, placement, meta, highlighted})
        : null;

      const cellStyle: React.CSSProperties = {
        width: cellSize,
        height: cellSize,
        border: `1px solid ${palette.boardCellBorder}`,
        background: highlighted
          ? palette.boardCellHighlight
          : activeCell
            ? palette.boardCellBg
            : 'rgba(148, 163, 184, 0.12)',
        fontWeight: highlighted ? 600 : 500,
        opacity: activeCell ? 1 : 0.55,
        cursor: activeCell ? 'default' : 'not-allowed',
        fontSize: tileFontSize,
      };

      cells.push(
        <div
          key={`${x}-${globalRow}`}
          data-testid="playground-board-cell"
          data-x={x}
          data-y={globalRow}
          data-active={activeCell ? '1' : '0'}
          className={styles.boardCell}
          style={cellStyle}
          onDragOver={event => {
            if (!activeCell) return;
            event.preventDefault();
          }}
          onDrop={event => {
            if (!activeCell) return;
            onDropCell(x, viewRow, event);
          }}
          onMouseEnter={() => {
            if (showMoves) {
              onHoverForMoves(x, globalRow);
            }
          }}
        >
          {symbol && <span>{symbol.slice(0, 2)}</span>}
          {typeof score === 'number' && !Number.isNaN(score) && (
            <span
              className={styles.boardCellScore}
              style={{fontSize: tileScoreFontSize, bottom: Math.max(2, Math.round(cellSize * 0.12)), right: Math.max(2, Math.round(cellSize * 0.12))}}
            >
              {score}
            </span>
          )}
          {overlay}
        </div>,
      );
    }
  }

  return (
    <div
      className={styles.boardGrid}
      style={{gridTemplateColumns: `repeat(${board.width}, ${cellSize}px)`, gap: cellGap, margin: '0 auto'}}
    >
      {cells}
    </div>
  );
};

export default PlaygroundBoard;
