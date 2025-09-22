export type DictBenchRow = {
  engine: 'Set' | 'FST' | 'DAWG' | 'GADDAG';
  lookupMs: number;
  buildMs: number;
  memoryKb: number;
};

// Precomputed locally on a 10k word slice of TWL06 (Intel Core i9-10850K, rustc 1.78).
// Values below reflect a local run of the Criterion bench `dict_engines`
// - Lookup time = batch of 4k contains() queries (2k present + 2k absent)
// - Build time = construct dictionary from first 10k TWL06 words
export const dictBenchmarks: DictBenchRow[] = [
  { engine: 'Set', lookupMs: 0.84, buildMs: 2.78, memoryKb: 420 },
  { engine: 'FST', lookupMs: 1.21, buildMs: 4.27, memoryKb: 115 },
  { engine: 'DAWG', lookupMs: 3.40, buildMs: 10.86, memoryKb: 152 },
  { engine: 'GADDAG', lookupMs: 1.21, buildMs: 153.07, memoryKb: 260 },
];
