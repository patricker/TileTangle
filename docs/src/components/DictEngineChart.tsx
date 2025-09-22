import React from 'react';
import {dictBenchmarks} from '../data/dictBench';

const barColor = {
  lookup: '#4F46E5',
  build: '#22C55E',
  memory: '#F97316',
} as const;

export default function DictEngineChart(): JSX.Element {
  const maxLookup = Math.max(...dictBenchmarks.map(row => row.lookupMs));
  const maxBuild = Math.max(...dictBenchmarks.map(row => row.buildMs));
  const maxMemory = Math.max(...dictBenchmarks.map(row => row.memoryKb));
  return (
    <div style={{border: '1px solid var(--ifm-color-emphasis-200)', borderRadius: 8, padding: 16}}>
      <div style={{fontSize: 14, marginBottom: 12, color: 'var(--ifm-color-emphasis-700)'}}>
        Offline benchmark snapshot (10k words, Intel Core i9-10850K). Lower lookup/build numbers are better.
      </div>
      <div style={{display: 'grid', gridTemplateColumns: '140px 1fr 1fr 1fr', gap: 12, fontSize: 14}}>
        <div style={{fontWeight: 600}}>Engine</div>
        <div style={{fontWeight: 600}}>Lookup (ms)</div>
        <div style={{fontWeight: 600}}>Build (ms)</div>
        <div style={{fontWeight: 600}}>Memory (KiB)</div>
        {dictBenchmarks.map(row => (
          <React.Fragment key={row.engine}>
            <div style={{fontWeight: 500}}>{row.engine}</div>
            <Bar value={row.lookupMs} max={maxLookup} color={barColor.lookup} formatter={v => v.toFixed(2)} />
            <Bar value={row.buildMs} max={maxBuild} color={barColor.build} formatter={v => v.toFixed(1)} />
            <Bar value={row.memoryKb} max={maxMemory} color={barColor.memory} formatter={v => v.toFixed(0)} />
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

type BarProps = {
  value: number;
  max: number;
  color: string;
  formatter: (v: number) => string;
};

function Bar({value, max, color, formatter}: BarProps): JSX.Element {
  const width = max === 0 ? 0 : Math.max(8, (value / max) * 100);
  return (
    <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
      <div style={{flexGrow: 1, background: 'var(--ifm-color-emphasis-200)', borderRadius: 4, height: 12, position: 'relative'}}>
        <div style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: `${width}%`,
          height: '100%',
          background: color,
          borderRadius: 4,
        }} />
      </div>
      <span style={{minWidth: 48, textAlign: 'right'}}>{formatter(value)}</span>
    </div>
  );
}
