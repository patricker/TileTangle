import React from 'react';
import styles from '../PlaygroundLayout.module.css';
import {StatChip, type StatChipProps} from './ui';

export type PlaygroundHeroProps = {
  eyebrow: string;
  title: React.ReactNode;
  description?: React.ReactNode;
  stats?: StatChipProps[];
  actions?: React.ReactNode;
  rightSlot?: React.ReactNode;
};

const PlaygroundHero: React.FC<PlaygroundHeroProps> = ({
  eyebrow,
  title,
  description,
  stats,
  actions,
  rightSlot,
}) => {
  return (
    <header className={styles.hero}>
      <div className={styles.heroCopy}>
        <div className={styles.heroEyebrow}>{eyebrow}</div>
        <h2 className={styles.heroTitle}>{title}</h2>
        {description && <p className={styles.heroDescription}>{description}</p>}
        {actions}
        {stats && stats.length > 0 && (
          <div className={styles.heroStatsRow}>
            {stats.map(stat => (
              <StatChip key={stat.label} label={stat.label} value={stat.value} />
            ))}
          </div>
        )}
      </div>
      {rightSlot && <div className={styles.heroRight}>{rightSlot}</div>}
    </header>
  );
};

export default PlaygroundHero;
