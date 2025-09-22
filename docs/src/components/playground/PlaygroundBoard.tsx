import React from 'react';
import styles from '../PlaygroundLayout.module.css';
import type {BoardJson} from './types';

type BoardPalette = {
  boardCellBorder: string;
  boardCellHighlight: string;
  boardCellBg: string;
  boardHexBorder?: string;
};

export const HEX_POLYGON = 'polygon(25% 0%, 75% 0%, 100% 50%, 75% 100%, 25% 100%, 0% 50%)';

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
  tileShape?: 'square' | 'hex';
  readonly?: boolean;
  onCellClick?: (x: number, y: number) => void;
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
  tileShape = 'square',
  readonly = false,
  onCellClick,
  renderOverlay,
}) => {
  if (!use3D && tileShape === 'hex') {
    return (
      <HexBoard
        board={board}
        layerHeight={layerHeight}
        currentLayer={currentLayer}
        cellSize={cellSize}
        cellGap={cellGap}
        tileFontSize={tileFontSize}
        tileScoreFontSize={tileScoreFontSize}
        highlightCells={highlightCells}
        isCellActive={isCellActive}
        cellDisplay={cellDisplay}
        getTileMeta={getTileMeta}
        onDropCell={onDropCell}
        showMoves={showMoves}
        onHoverForMoves={onHoverForMoves}
        palette={palette}
        readonly={readonly}
        onCellClick={onCellClick}
        renderOverlay={renderOverlay}
      />
    );
  }

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
        ? renderOverlay({x, viewY: viewRow, globalY: globalRow, placement, meta, highlighted})
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
        cursor: activeCell
          ? (onCellClick && !readonly ? 'pointer' : 'default')
          : 'not-allowed',
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
            if (!activeCell || readonly) return;
            event.preventDefault();
          }}
          onDrop={event => {
            if (!activeCell || readonly) return;
            onDropCell(x, viewRow, event);
          }}
          onMouseEnter={() => {
            if (showMoves) {
              onHoverForMoves(x, globalRow);
            }
          }}
          onClick={() => {
            if (activeCell && onCellClick) onCellClick(x, globalRow);
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

type HexBoardProps = {
  board: BoardJson;
  layerHeight: number;
  currentLayer: number;
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
  readonly?: boolean;
  onCellClick?: (x: number, y: number) => void;
  renderOverlay?: PlaygroundBoardProps['renderOverlay'];
};

const HEX_HEIGHT_RATIO = Math.sqrt(3) / 2;

function HexBoard({
  board,
  layerHeight,
  currentLayer,
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
  readonly = false,
  onCellClick,
  renderOverlay,
}: HexBoardProps): JSX.Element {
  const hexWidth = cellSize;
  const hexHeight = Math.max(12, Math.round(cellSize * HEX_HEIGHT_RATIO));
  const horizontalStep = hexWidth * 0.75 + cellGap;
  const rowOffset = (hexWidth + cellGap) / 2;
  const verticalStep = hexHeight + cellGap;

  const containerWidth = (board.width - 1) * horizontalStep + hexWidth + (layerHeight > 1 ? rowOffset : 0);
  const containerHeight = (layerHeight - 1) * verticalStep + hexHeight;

  const cells: React.ReactNode[] = [];

  for (let viewRow = 0; viewRow < layerHeight; viewRow++) {
    const globalRow = viewRow + currentLayer * layerHeight;
    for (let x = 0; x < board.width; x++) {
      const activeCell = isCellActive(x, globalRow);
      const placement = activeCell ? cellDisplay(x, viewRow) : '';
      const meta = placement ? getTileMeta(placement) : undefined;
      const symbol = meta?.symbol ?? placement?.slice(0, 2) ?? '';
      const score = meta?.score;
      const highlighted = activeCell && highlightCells.has(`${x},${globalRow}`);
      const overlay = activeCell && renderOverlay
        ? renderOverlay({x, viewY: viewRow, globalY: globalRow, placement, meta, highlighted})
        : null;

      const left = x * horizontalStep + (globalRow % 2 !== 0 ? rowOffset : 0);
      const top = viewRow * verticalStep;
      const scoreOffset = Math.max(2, Math.round(hexWidth * 0.12));
      const baseBorder = palette.boardHexBorder ?? palette.boardCellBorder;
      const highlightBorder = palette.boardCellHighlight;

      const style: React.CSSProperties = {
        width: hexWidth,
        height: hexHeight,
        left,
        top,
        clipPath: HEX_POLYGON,
        background: highlighted ? palette.boardCellHighlight : palette.boardCellBg,
        border: `1.5px solid ${highlighted ? highlightBorder : baseBorder}`,
        boxSizing: 'border-box',
        boxShadow: `0 0 0 1px ${highlighted ? highlightBorder : baseBorder}, 0 6px 14px rgba(15, 23, 42, 0.18)`,
        fontSize: tileFontSize,
        transition: 'border-color 120ms ease, background-color 120ms ease, transform 120ms ease',
      };

      if (!activeCell) {
        style.background = 'rgba(148, 163, 184, 0.12)';
        style.border = '1.5px solid rgba(148, 163, 184, 0.3)';
        style.boxShadow = '0 0 0 1px rgba(148, 163, 184, 0.28)';
        style.opacity = 0.55;
        style.pointerEvents = 'none';
      }

      cells.push(
        <div
          key={`${x}-${globalRow}`}
          data-testid="playground-board-cell"
          data-x={x}
          data-y={globalRow}
          data-active={activeCell ? '1' : '0'}
          className={styles.hexCell}
          style={style}
          onDragOver={event => {
            if (!activeCell || readonly) return;
            event.preventDefault();
          }}
          onDrop={event => {
            if (!activeCell || readonly) return;
            onDropCell(x, viewRow, event);
          }}
          onMouseEnter={() => {
            if (showMoves && activeCell) {
              onHoverForMoves(x, globalRow);
            }
          }}
          onClick={() => {
            if (activeCell && onCellClick) onCellClick(x, globalRow);
          }}
        >
          {symbol && <span className={styles.hexCellLabel}>{symbol.slice(0, 2)}</span>}
          {typeof score === 'number' && !Number.isNaN(score) && (
            <span
              className={styles.boardCellScore}
              style={{fontSize: tileScoreFontSize, bottom: scoreOffset, right: scoreOffset}}
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
    <div className={styles.hexBoard} style={{width: containerWidth, height: containerHeight}}>
      {cells}
    </div>
  );
}
