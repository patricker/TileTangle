export type ClassicTileset = {
  tile_kinds: {id: string; symbol: string; score: number; is_blank?: boolean; aliases?: string[]}[];
  tile_counts: Record<string, number>;
};

export function classicTilesets(): ClassicTileset {
  const entries: {id: string; symbol: string; score: number; count: number; is_blank?: boolean}[] = [
    {id: 'A', symbol: 'A', score: 1, count: 9},
    {id: 'B', symbol: 'B', score: 3, count: 2},
    {id: 'C', symbol: 'C', score: 3, count: 2},
    {id: 'D', symbol: 'D', score: 2, count: 4},
    {id: 'E', symbol: 'E', score: 1, count: 12},
    {id: 'F', symbol: 'F', score: 4, count: 2},
    {id: 'G', symbol: 'G', score: 2, count: 3},
    {id: 'H', symbol: 'H', score: 4, count: 2},
    {id: 'I', symbol: 'I', score: 1, count: 9},
    {id: 'J', symbol: 'J', score: 8, count: 1},
    {id: 'K', symbol: 'K', score: 5, count: 1},
    {id: 'L', symbol: 'L', score: 1, count: 4},
    {id: 'M', symbol: 'M', score: 3, count: 2},
    {id: 'N', symbol: 'N', score: 1, count: 6},
    {id: 'O', symbol: 'O', score: 1, count: 8},
    {id: 'P', symbol: 'P', score: 3, count: 2},
    {id: 'Q', symbol: 'Q', score: 10, count: 1},
    {id: 'R', symbol: 'R', score: 1, count: 6},
    {id: 'S', symbol: 'S', score: 1, count: 4},
    {id: 'T', symbol: 'T', score: 1, count: 6},
    {id: 'U', symbol: 'U', score: 1, count: 4},
    {id: 'V', symbol: 'V', score: 4, count: 2},
    {id: 'W', symbol: 'W', score: 4, count: 2},
    {id: 'X', symbol: 'X', score: 8, count: 1},
    {id: 'Y', symbol: 'Y', score: 4, count: 2},
    {id: 'Z', symbol: 'Z', score: 10, count: 1},
    {id: 'BL', symbol: '_', score: 0, count: 2, is_blank: true},
  ];
  const tile_counts: Record<string, number> = {};
  const tile_kinds = entries.map(({id, symbol, score, count, is_blank}) => {
    tile_counts[id] = count;
    return {id, symbol, score, is_blank: !!is_blank};
  });
  return {tile_kinds, tile_counts};
}

export function classicBonuses(): {x: number; y: number; letter_mul?: number; word_mul?: number; tags?: string[]}[] {
  const TW = [[0, 0], [0, 7], [0, 14], [7, 0], [7, 14], [14, 0], [14, 7], [14, 14]];
  const DW = [[1, 1], [2, 2], [3, 3], [4, 4], [7, 7], [10, 10], [11, 11], [12, 12], [13, 13], [13, 1], [12, 2], [11, 3], [10, 4], [1, 13], [2, 12], [3, 11], [4, 10]];
  const TL = [[5, 1], [9, 1], [1, 5], [5, 5], [9, 5], [13, 5], [1, 9], [5, 9], [9, 9], [13, 9], [5, 13], [9, 13]];
  const DL = [[3, 0], [11, 0], [6, 2], [8, 2], [0, 3], [7, 3], [14, 3], [2, 6], [6, 6], [8, 6], [12, 6], [3, 7], [11, 7], [2, 8], [6, 8], [8, 8], [12, 8], [0, 11], [7, 11], [14, 11], [6, 12], [8, 12], [3, 14], [11, 14]];
  const out: {x: number; y: number; letter_mul?: number; word_mul?: number; tags?: string[]}[] = [];
  TW.forEach(([x, y]) => out.push({x, y, word_mul: 3}));
  DW.forEach(([x, y]) => out.push({x, y, word_mul: 2}));
  TL.forEach(([x, y]) => out.push({x, y, letter_mul: 3}));
  DL.forEach(([x, y]) => out.push({x, y, letter_mul: 2}));
  return out;
}
