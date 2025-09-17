import React, {useState} from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';

type SnapshotRow = {
  id: string;
  label: string;
  baseline_ns: number;
  current_ns: number;
};

type Snapshot = {
  generated_at: string;
  unit: string;
  benchmarks: SnapshotRow[];
};

type Status = 'idle' | 'loading' | 'loaded' | 'error';

function formatDuration(ns: number): string {
  if (ns >= 1_000_000) {
    return `${(ns / 1_000_000).toFixed(2)} ms`;
  }
  if (ns >= 1_000) {
    return `${(ns / 1_000).toFixed(2)} µs`;
  }
  return `${ns.toFixed(2)} ns`;
}

function ratioText(baseline: number, current: number): string {
  if (baseline === 0) {
    return '—';
  }
  const ratio = current / baseline;
  return `${ratio.toFixed(2)}×`;
}

function deltaText(baseline: number, current: number): string {
  const delta = current - baseline;
  const sign = delta >= 0 ? '+' : '−';
  return `${sign} ${formatDuration(Math.abs(delta))}`;
}

export default function BenchmarkSnapshot(): JSX.Element {
  const snapshotUrl = useBaseUrl('benchmarks/core.json');
  const [status, setStatus] = useState<Status>('idle');
  const [rows, setRows] = useState<SnapshotRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  const handleLoad = async () => {
    setStatus('loading');
    setError(null);
    try {
      const response = await fetch(snapshotUrl);
      if (!response.ok) {
        throw new Error(`Request failed with status ${response.status}`);
      }
      const payload: Snapshot = await response.json();
      setRows(payload.benchmarks);
      setStatus('loaded');
    } catch (err) {
      setStatus('error');
      setError(err instanceof Error ? err.message : 'Unknown error');
    }
  };

  return (
    <div style={{border: '1px solid var(--ifm-color-emphasis-200)', borderRadius: 8, padding: 16}}>
      <div style={{display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12}}>
        <button type="button" className="button button--primary" onClick={handleLoad} disabled={status === 'loading'}>
          {status === 'loading' ? 'Loading…' : 'Load snapshot'}
        </button>
        <span style={{fontSize: 14, color: 'var(--ifm-color-emphasis-700)'}}>
          Pre-recorded run from CI showing baseline vs current Criterion timings.
        </span>
      </div>
      {status === 'error' && error && (
        <div style={{color: 'var(--ifm-color-danger)'}}>Failed to load snapshot: {error}</div>
      )}
      {status === 'loaded' && (
        <div style={{overflowX: 'auto'}}>
          <table style={{width: '100%', borderCollapse: 'collapse', fontSize: 14}}>
            <thead>
              <tr style={{textAlign: 'left'}}>
                <th style={{padding: '8px 4px'}}>Benchmark</th>
                <th style={{padding: '8px 4px', textAlign: 'right'}}>Baseline</th>
                <th style={{padding: '8px 4px', textAlign: 'right'}}>Current</th>
                <th style={{padding: '8px 4px', textAlign: 'right'}}>Δ</th>
                <th style={{padding: '8px 4px', textAlign: 'right'}}>Ratio</th>
              </tr>
            </thead>
            <tbody>
              {rows.map(row => (
                <tr key={row.id} style={{borderTop: '1px solid var(--ifm-table-border-color)'}}>
                  <td style={{padding: '8px 4px'}}>{row.label}</td>
                  <td style={{padding: '8px 4px', textAlign: 'right'}}>{formatDuration(row.baseline_ns)}</td>
                  <td style={{padding: '8px 4px', textAlign: 'right'}}>{formatDuration(row.current_ns)}</td>
                  <td style={{padding: '8px 4px', textAlign: 'right'}}>{deltaText(row.baseline_ns, row.current_ns)}</td>
                  <td style={{padding: '8px 4px', textAlign: 'right'}}>{ratioText(row.baseline_ns, row.current_ns)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {status === 'idle' && (
        <div style={{fontSize: 14, color: 'var(--ifm-color-emphasis-600)'}}>
          Click the button to fetch a canned run captured from the `perf` CI job.
        </div>
      )}
    </div>
  );
}
