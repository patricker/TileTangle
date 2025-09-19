import React from 'react';
import styles from '../PlaygroundLayout.module.css';

export type PanelProps = {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
  accent?: boolean;
  density?: 'spacious' | 'compact';
  children: React.ReactNode;
};

export function Panel({title, subtitle, actions, accent, density = 'spacious', children}: PanelProps): JSX.Element {
  const bodyClass = density === 'compact' ? styles.panelBodyCompact : styles.panelBody;
  return (
    <section className={`${styles.panel} ${accent ? styles.panelAccent : ''}`}>
      <div className={styles.panelHeader}>
        <div>
          <div className={styles.panelTitle}>{title}</div>
          {subtitle && <div className={styles.panelSubtitle}>{subtitle}</div>}
        </div>
        {actions && <div className={styles.panelActions}>{actions}</div>}
      </div>
      <div className={bodyClass}>{children}</div>
    </section>
  );
}

export type StatChipProps = {
  label: string;
  value: React.ReactNode;
};

export function StatChip({label, value}: StatChipProps): JSX.Element {
  return (
    <div className={styles.statChip}>
      <span className={styles.statChipLabel}>{label}</span>
      <span className={styles.statChipValue}>{value}</span>
    </div>
  );
}

export type SegmentedOption = {
  value: string;
  label: string;
  hint?: string;
  testId?: string;
};

export type SegmentedControlProps = {
  name: string;
  value: string;
  options: SegmentedOption[];
  onChange: (value: string) => void;
};

export function SegmentedControl({name, value, options, onChange}: SegmentedControlProps): JSX.Element {
  return (
    <div className={styles.segmentedControl} role="radiogroup" aria-label={name}>
      {options.map(option => {
        const active = option.value === value;
        return (
          <label
            key={option.value}
            className={styles.segmentedControlOption}
            data-active={active ? '1' : '0'}
          >
            <input
              type="radio"
              name={`segmented-${name}`}
              value={option.value}
              data-testid={option.testId}
              checked={active}
              onChange={() => onChange(option.value)}
            />
            <span>{option.label}</span>
            {option.hint && <small>{option.hint}</small>}
          </label>
        );
      })}
    </div>
  );
}

export type RackTileProps = {
  symbol: string;
  score?: number;
  size: number;
  fontSize: number;
  scoreFontSize: number;
  draggable?: boolean;
  onDragStart?: (event: React.DragEvent<HTMLDivElement>) => void;
  onClick?: (event: React.MouseEvent<HTMLDivElement>) => void;
  dataKind?: string;
  testId?: string;
  style?: React.CSSProperties;
  className?: string;
  ariaPressed?: boolean;
};

export function RackTile({
  symbol,
  score,
  size,
  fontSize,
  scoreFontSize,
  draggable,
  onDragStart,
  onClick,
  dataKind,
  testId,
  style,
  className,
  ariaPressed,
}: RackTileProps): JSX.Element {
  const classNames = className ? `${styles.rackTile} ${className}` : styles.rackTile;
  const displaySymbol = symbol.slice(0, 2);
  return (
    <div
      className={classNames}
      style={{width: size, height: size, fontSize, ...style}}
      draggable={!!draggable}
      data-kind={dataKind}
      data-testid={testId}
      onDragStart={draggable ? onDragStart : undefined}
      onClick={onClick}
      role={onClick ? 'button' : undefined}
      aria-pressed={onClick ? ariaPressed ?? false : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      <span>{displaySymbol}</span>
      {typeof score === 'number' && !Number.isNaN(score) && (
        <span className={styles.rackTileScore} style={{fontSize: scoreFontSize}}>
          {score}
        </span>
      )}
    </div>
  );
}

export type MoveCardAction = {
  label: string;
  onClick?: () => void;
  disabled?: boolean;
  testId?: string;
};

export type MoveCardProps = {
  index: number;
  word: string;
  score?: number;
  testId?: string;
  actions?: MoveCardAction[];
  children?: React.ReactNode;
};

export function MoveCard({index, word, score, testId, actions, children}: MoveCardProps): JSX.Element {
  return (
    <div className={styles.moveCard} data-testid={testId}>
      <div className={styles.moveHeader}>
        <span>#{index + 1}</span>
        <strong>{word}</strong>
        {score != null && <span className={styles.moveScore}>{score} pts</span>}
      </div>
      {children}
      {actions && actions.length > 0 && (
        <div className={styles.buttonRow}>
          {actions.map(action => (
            <button
              key={action.label}
              type="button"
              onClick={action.onClick}
              disabled={action.disabled}
              data-testid={action.testId}
            >
              {action.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export type RackRowTile = RackTileProps & {key: React.Key};

export type RackRowProps = {
  tiles: RackRowTile[];
  emptyMessage?: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
};

export function RackRow({tiles, emptyMessage, footer, className}: RackRowProps): JSX.Element {
  const emptyContent =
    tiles.length === 0 && emptyMessage
      ? React.isValidElement(emptyMessage)
        ? emptyMessage
        : <div className={styles.helperText}>{emptyMessage}</div>
      : null;

  return (
    <>
      <div className={className ? `${styles.rackRow} ${className}` : styles.rackRow}>
        {tiles.map(tile => {
          const {key, ...tileProps} = tile;
          return <RackTile key={key} {...tileProps} />;
        })}
        {emptyContent}
      </div>
      {footer}
    </>
  );
}

export type MoveListItem = {
  key: React.Key;
  word: string;
  score?: number;
  actions?: MoveCardAction[];
  testId?: string;
  body?: React.ReactNode;
};

export type MoveListProps = {
  items: MoveListItem[];
  emptyMessage?: React.ReactNode;
};

export function MoveList({items, emptyMessage}: MoveListProps): JSX.Element {
  if (items.length === 0) {
    if (!emptyMessage) {
      return <div className={styles.helperText}>No moves available.</div>;
    }
    return React.isValidElement(emptyMessage)
      ? emptyMessage
      : <div className={styles.helperText}>{emptyMessage}</div>;
  }

  return (
    <div className={styles.movesList}>
      {items.map(item => {
        const {key, body, ...rest} = item;
        return (
          <MoveCard key={key} {...rest}>
            {body}
          </MoveCard>
        );
      })}
    </div>
  );
}

export type ButtonConfig = {
  key?: React.Key;
  label: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  testId?: string;
};

export type ButtonRowProps = {
  buttons: ButtonConfig[];
  className?: string;
};

export function ButtonRow({buttons, className}: ButtonRowProps): JSX.Element {
  if (buttons.length === 0) {
    return <div className={className ?? styles.buttonRow} />;
  }
  return (
    <div className={className ? `${styles.buttonRow} ${className}` : styles.buttonRow}>
      {buttons.map((button, index) => (
        <button
          key={button.key ?? index}
          type="button"
          onClick={button.onClick}
          disabled={button.disabled}
          data-testid={button.testId}
        >
          {button.label}
        </button>
      ))}
    </div>
  );
}
