import React, {useMemo, useState} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import MiniBoard from './MiniBoard';
import type {BoardJson} from '../playground/types';
import {buildShapeMask} from '../playground/config';
import {resolveBonusPreset, type AdjacencyMode, type BonusCell} from '../playground/bonuses';
import styles from '../PlaygroundLayout.module.css';
import BonusLegend from './BonusLegend';
import {getPlaygroundPalette} from '../playground/theme';

type Preset = {
  label: string;
  shape: 'rect' | 'diamond' | 'cross' | 'hexagon' | 'triangle' | 'ring';
  width: number;
  height: number;
  adjacency?: AdjacencyMode;
  tileShape?: 'square' | 'hex';
};

export type GeometryGalleryProps = {
  presets?: Preset[];
  showBonuses?: boolean;
};

const defaultPresets: Preset[] = [
  {label: 'Rect', shape: 'rect', width: 9, height: 9, adjacency: 'orthogonal', tileShape: 'square'},
  {label: 'Diamond', shape: 'diamond', width: 11, height: 11, adjacency: 'orthogonal', tileShape: 'square'},
  {label: 'Hex', shape: 'rect', width: 9, height: 9, adjacency: 'hex', tileShape: 'hex'},
  {label: 'Triangle', shape: 'triangle', width: 11, height: 11, adjacency: 'orthogonal', tileShape: 'square'},
  {label: 'Ring', shape: 'ring', width: 11, height: 11, adjacency: 'orthogonal', tileShape: 'square'},
];

export default function GeometryGallery({presets = defaultPresets, showBonuses = true}: GeometryGalleryProps): JSX.Element {
  const {colorMode} = useColorMode();
  const palette = useMemo(() => getPlaygroundPalette(colorMode as 'light' | 'dark'), [colorMode]);
  const [activeIndex, setActiveIndex] = useState(0);
  const preset = presets[Math.max(0, Math.min(presets.length - 1, activeIndex))];

  const mask = useMemo(() => buildShapeMask(preset.width, preset.height, preset.shape), [preset.width, preset.height, preset.shape]);

  const board: BoardJson = useMemo(() => ({
    width: preset.width,
    height: preset.height,
    rows: Array.from({length: preset.height}, () => Array.from({length: preset.width}, () => '')),
  }), [preset.width, preset.height]);

  const bonusMap = useMemo(() => {
    if (!showBonuses) return new Map<string, BonusCell>();
    const list = resolveBonusPreset('auto', preset.shape, preset.width, preset.height, preset.adjacency ?? 'orthogonal');
    const map = new Map<string, BonusCell>();
    for (const b of list) map.set(`${b.x},${b.y}`, b);
    return map;
  }, [showBonuses, preset]);

  return (
    <div className={styles.galleryLayout}>
      <div className={styles.gallerySidebar}>
        <div className={styles.modeLabel}>Board Shapes</div>
        <div
          className={styles.galleryButtonList}
          role="radiogroup"
          aria-label="Board shape"
        >
          {presets.map((p, i) => (
            <button
              key={p.label}
              type="button"
              className={styles.choiceButton}
              data-active={i === activeIndex ? '1' : '0'}
              aria-pressed={i === activeIndex}
              onClick={() => setActiveIndex(i)}
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>
      <div className={styles.galleryPreview}>
        <div className={styles.boardWrapper}>
          <MiniBoard
            board={board}
            tileMeta={() => undefined}
            tileShape={preset.tileShape ?? 'square'}
            overlay={({x, y}) => {
              const active = mask.has(`${x},${y}`);
              if (!active) {
                return <div className={styles.hintBadge}>off</div>;
              }
              const bonus = bonusMap.get(`${x},${y}`);
              if (!bonus) return null;
              const label = bonus.word_mul && bonus.word_mul > 1 ? `${bonus.word_mul}W` : (bonus.letter_mul && bonus.letter_mul > 1 ? `${bonus.letter_mul}L` : '');
              if (!label) return null;
              const tone = bonus.word_mul && bonus.word_mul > 1 ? 'word' : 'letter';
              const cls = tone === 'word' ? styles.bonusChipWord : styles.bonusChipLetter;
              return <div className={`${styles.bonusChip} ${cls}`}>{label}</div>;
            }}
            highlight={new Set<string>()}
            cellSize={28}
            cellGap={4}
          />
        </div>
        <div className={styles.helperText} style={{marginTop: 8}}>
          {preset.tileShape === 'hex'
            ? 'Hex adjacency uses staggered rows; labels indicate bonus cells.'
            : 'Inactive cells show as dimmed; labels indicate 2L/3L/2W/3W bonuses.'}
        </div>
        {showBonuses && <BonusLegend />}
      </div>
    </div>
  );
}
