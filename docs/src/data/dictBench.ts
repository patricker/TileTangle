export type DictBenchRow = {
  engine: 'Set' | 'FST' | 'DAWG' | 'GADDAG';
  lookupMs: number;
  buildMs: number;
  memoryKb: number;
};

// Precomputed locally on a 10k word slice of TWL06 (M1 Pro, rustc 1.78).
export const dictBenchmarks: DictBenchRow[] = [
  { engine: 'Set', lookupMs: 0.42, buildMs: 31.5, memoryKb: 420 },
  { engine: 'FST', lookupMs: 0.08, buildMs: 58.4, memoryKb: 115 },
  { engine: 'DAWG', lookupMs: 0.12, buildMs: 64.2, memoryKb: 152 },
  { engine: 'GADDAG', lookupMs: 0.15, buildMs: 92.7, memoryKb: 260 },
];
