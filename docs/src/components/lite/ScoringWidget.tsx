import React, {useMemo, useState} from 'react';
import {classicTilesets} from '../demoUtils';
import {Panel} from '../playground/ui';

type Mult = 'none' | 'DL' | 'TL';
type WordMult = 'x1' | 'x2' | 'x3';

export default function ScoringWidget(): JSX.Element {
  const {tile_kinds} = useMemo(() => classicTilesets(), []);
  const baseScores = useMemo(() => {
    const map = new Map<string, number>();
    for (const k of tile_kinds) map.set(k.id, k.score);
    return map;
  }, [tile_kinds]);

  const [word, setWord] = useState('READ');
  const [letterMults, setLetterMults] = useState<Mult[]>(['none', 'none', 'none', 'none']);
  const [wordMult, setWordMult] = useState<WordMult>('x2');

  const letters = useMemo(() => word.toUpperCase().replace(/[^A-Z]/g, '').split(''), [word]);
  const totals = useMemo(() => {
    const ls: number[] = [];
    letters.forEach((ch, idx) => {
      const base = baseScores.get(ch) ?? 0;
      const mul = (letterMults[idx] ?? 'none');
      const factor = mul === 'TL' ? 3 : mul === 'DL' ? 2 : 1;
      ls.push(base * factor);
    });
    const wordMul = wordMult === 'x3' ? 3 : wordMult === 'x2' ? 2 : 1;
    const subtotal = ls.reduce((a, b) => a + b, 0);
    const total = subtotal * wordMul;
    return {ls, subtotal, total};
  }, [baseScores, letterMults, letters, wordMult]);

  return (
    <Panel title="Scoring (lite)" subtitle="Per-letter multipliers and word bonus." density="compact">
      <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
        <label style={{fontSize: 14}}>
          Word
          <input
            type="text"
            value={word}
            onChange={e => { setWord(e.target.value); setLetterMults([]); }}
            style={{marginLeft: 8, padding: '4px 8px'}}
            placeholder="READ"
            maxLength={12}
          />
        </label>
        {letters.length > 0 && (
          <div style={{display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap'}}>
            {letters.map((ch, idx) => (
              <LetterMult
                key={`${ch}-${idx}`}
                ch={ch}
                base={baseScores.get(ch) ?? 0}
                value={letterMults[idx] ?? 'none'}
                onChange={m => {
                  const next = letterMults.slice();
                  next[idx] = m;
                  setLetterMults(next);
                }}
              />
            ))}
          </div>
        )}
        <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
          <label style={{fontSize: 14}}>
            Word bonus
            <select value={wordMult} onChange={e => setWordMult(e.target.value as WordMult)} style={{marginLeft: 8}}>
              <option value="x1">×1</option>
              <option value="x2">×2</option>
              <option value="x3">×3</option>
            </select>
          </label>
          <div style={{marginLeft: 'auto', fontWeight: 600}}>
            Total: {totals.total} pts (letters {totals.subtotal})
          </div>
        </div>
      </div>
    </Panel>
  );
}

function LetterMult({ch, base, value, onChange}: {ch: string; base: number; value: Mult; onChange: (v: Mult) => void}) {
  return (
    <div style={{display: 'inline-flex', alignItems: 'center', gap: 6, border: '1px solid var(--ifm-color-emphasis-300)', borderRadius: 8, padding: '6px 8px'}}>
      <strong>{ch}</strong>
      <span style={{opacity: 0.7}}>({base})</span>
      <select value={value} onChange={e => onChange(e.target.value as Mult)}>
        <option value="none">—</option>
        <option value="DL">DL</option>
        <option value="TL">TL</option>
      </select>
    </div>
  );
}

