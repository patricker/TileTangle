import React, {useCallback, useMemo, useRef, useState} from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {Panel, ButtonRow} from '../playground/ui';

async function fetchText(url: string): Promise<string | null> {
  try {
    const resp = await fetch(url);
    if (resp.ok) return await resp.text();
  } catch (_) {
    // ignore
  }
  return null;
}

export default function WordCheck(): JSX.Element {
  const [word, setWord] = useState('READ');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<'valid' | 'invalid' | null>(null);
  const dictRef = useRef<Set<string> | null>(null);

  const ensureDict = useCallback(async () => {
    if (dictRef.current) return;
    setLoading(true);
    setError(null);
    try {
      const txt = (await fetchText(txtUrl1)) || (await fetchText(txtUrl2));
      if (!txt) throw new Error('Failed to load demo dictionary');
      const set = new Set<string>();
      txt.split(/\r?\n/).forEach(line => {
        const w = line.trim();
        if (!w || w.startsWith('#')) return;
        set.add(w.toUpperCase());
      });
      dictRef.current = set;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [txtUrl1, txtUrl2]);

  const check = useCallback(async () => {
    await ensureDict();
    const s = dictRef.current;
    if (!s) return;
    const query = word.replace(/[^\p{L}]/gu, '').toUpperCase();
    if (!query) {
      setResult(null);
      return;
    }
    setResult(s.has(query) ? 'valid' : 'invalid');
  }, [ensureDict, word]);

  const buttons = useMemo(() => ([{key: 'check', label: loading ? 'Checking…' : 'Check', onClick: check, disabled: loading || !word.trim()}]), [check, loading, word]);

  return (
    <Panel title="Word check (lite)" subtitle="Text-based set membership (demo lexicon)." density="compact">
      <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
        <label style={{fontSize: 14}}>
          Word
          <input
            type="text"
            value={word}
            onChange={e => setWord(e.target.value)}
            style={{marginLeft: 8, padding: '4px 8px'}}
            placeholder="READ"
            maxLength={24}
          />
        </label>
        <ButtonRow buttons={buttons} />
      </div>
      {error && <div style={{color: 'var(--ifm-color-danger)', marginTop: 8}}>{error}</div>}
      {result && !error && (
        <div style={{marginTop: 8, fontWeight: 600, color: result === 'valid' ? 'var(--ifm-color-success)' : 'var(--ifm-color-danger)'}}>
          {result === 'valid' ? '✓ In dictionary' : '✕ Not in dictionary'}
        </div>
      )}
      {!result && !error && (
        <div style={{marginTop: 8, fontSize: 13, opacity: 0.7}}>Type a word and click Check. Uses a small demo lexicon for speed.</div>
      )}
    </Panel>
  );
}
  const txtUrl1 = useBaseUrl('dictionaries/TWL06.txt');
  const txtUrl2 = useBaseUrl('dictionaries/demo.txt');
