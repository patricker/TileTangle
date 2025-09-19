export type BoardShape = 'rect' | 'diamond' | 'cross' | 'hexagon' | 'triangle' | 'ring';
export type BonusPreset = 'auto' | 'none' | 'classic' | 'hex' | 'triangle' | 'ring';

export type SetupState = {
  width: number;
  height: number;
  depth: number;
  rackSize: number;
  tileCountsText: string;
  tileScoresText: string;
  shape: BoardShape;
  bonusPreset: BonusPreset;
};

type ParseResult = {
  map: Record<string, number>;
  error: string | null;
};

export const clamp = (value: number, min: number, max: number): number => {
  if (!Number.isFinite(value)) return min;
  return Math.min(Math.max(value, min), max);
};

export const formatTileCounts = (counts: Record<string, number>): string => {
  return Object.entries(counts)
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([id, count]) => `${id}:${count}`)
    .join('\n');
};

export const parseTileCounts = (text: string, fallback: Record<string, number>): ParseResult => {
  const trimmed = text.trim();
  if (!trimmed) {
    return {map: fallback, error: 'Tile pool cannot be empty.'};
  }
  const map: Record<string, number> = {};
  const tokens = trimmed.split(/[\n,]+/);
  for (const token of tokens) {
    const entry = token.trim();
    if (!entry) continue;
    const match = entry.match(/^([A-Za-z_]+)\s*[:=]\s*(\d+)$/);
    if (!match) {
      return {map: fallback, error: `Invalid tile entry: “${entry}”`};
    }
    const id = match[1].toUpperCase();
    const count = Number.parseInt(match[2], 10);
    if (!Number.isFinite(count) || count < 0) {
      return {map: fallback, error: `Invalid count for ${id}`};
    }
    map[id] = count;
  }
  if (Object.keys(map).length === 0) {
    return {map: fallback, error: 'Tile pool must include at least one tile.'};
  }
  return {map, error: null};
};

export const formatTileScores = (scores: Record<string, number>): string => {
  return Object.entries(scores)
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([id, score]) => `${id}:${score}`)
    .join('\n');
};

export const parseTileScores = (text: string, fallback: Record<string, number>): ParseResult => {
  const map: Record<string, number> = {...fallback};
  const trimmed = text.trim();
  if (!trimmed) {
    return {map, error: 'Tile scores cannot be empty.'};
  }
  const tokens = trimmed.split(/[\n,]+/);
  for (const token of tokens) {
    const entry = token.trim();
    if (!entry) continue;
    const match = entry.match(/^([A-Za-z_]+)\s*[:=]\s*(-?\d+)$/);
    if (!match) {
      return {map, error: `Invalid score entry: “${entry}”`};
    }
    const id = match[1].toUpperCase();
    const score = Number.parseInt(match[2], 10);
    if (!Number.isFinite(score)) {
      return {map, error: `Invalid score for ${id}`};
    }
    map[id] = score;
  }
  return {map, error: null};
};

export const buildShapeMask = (width: number, height: number, shape: BoardShape): Set<string> => {
  const mask = new Set<string>();
  if (width <= 0 || height <= 0) {
    return mask;
  }
  const midX = Math.floor((width - 1) / 2);
  const midY = Math.floor((height - 1) / 2);
  const diamondRadius = Math.floor(Math.min(width, height) / 2);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const key = `${x},${y}`;
      if (shape === 'rect') {
        mask.add(key);
        continue;
      }
      if (shape === 'diamond') {
        const dist = Math.abs(x - midX) + Math.abs(y - midY);
        if (dist <= diamondRadius) {
          mask.add(key);
        }
        continue;
      }
      if (shape === 'cross') {
        if (x === midX || y === midY) {
          mask.add(key);
        }
        continue;
      }
      if (shape === 'hexagon') {
        const dx = x - midX;
        const dy = y - midY;
        if (Math.abs(dx) + Math.abs(dy) + Math.abs(dx + dy) <= Math.floor(Math.min(width, height) * 1.5)) {
          mask.add(key);
        }
        continue;
      }
      if (shape === 'triangle') {
        const relY = y - Math.min(midY, midX);
        if (relY >= 0) {
          const span = width - relY * 2;
          if (span > 0) {
            const left = Math.floor((width - span) / 2);
            if (x >= left && x < left + span) {
              mask.add(key);
            }
          }
        }
        continue;
      }
      if (shape === 'ring') {
        const dx = x - midX;
        const dy = y - midY;
        const dist = Math.sqrt(dx * dx + dy * dy);
        const outer = Math.min(midX, midY) + 0.5;
        const inner = Math.max(outer - 2, 0);
        if (dist <= outer && dist >= inner) {
          mask.add(key);
        }
      }
    }
  }
  if (mask.size === 0) {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        mask.add(`${x},${y}`);
      }
    }
  }
  return mask;
};
