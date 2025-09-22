import React from 'react';
import styles from '../PlaygroundLayout.module.css';

export default function BonusLegend(): JSX.Element {
  return (
    <div style={{display: 'flex', alignItems: 'center', gap: 10, marginTop: 8, flexWrap: 'wrap'}}>
      <Chip label="2W" kind="word" />
      <Chip label="3W" kind="word" />
      <Chip label="2L" kind="letter" />
      <Chip label="3L" kind="letter" />
      <span className={styles.helperText}>W = word multiplier, L = letter multiplier.</span>
    </div>
  );
}

function Chip({label, kind}: {label: string; kind: 'word'|'letter'}) {
  const cls = kind === 'word' ? styles.bonusChipWord : styles.bonusChipLetter;
  return (
    <div className={`${styles.bonusChip} ${cls}`} style={{position: 'relative', inset: 'unset', width: 36, height: 24}}>
      {label}
    </div>
  );
}

