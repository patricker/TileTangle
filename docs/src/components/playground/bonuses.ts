import type {BoardShape, BonusPreset} from './config';

export type BonusCell = {
  x: number;
  y: number;
  letter_mul?: number;
  word_mul?: number;
  tags?: string[];
};

export type AdjacencyMode = 'orthogonal' | 'diagonal' | 'hex';

type BonusMap = Map<string, BonusCell>;

type BonusKind = 'classic' | 'hex' | 'triangle' | 'ring';

const keyFor = (x: number, y: number): string => `${x},${y}`;

const setWord = (map: BonusMap, x: number, y: number, mul: number, tags?: string[]) => {
  const key = keyFor(x, y);
  const existing = map.get(key);
  if (existing?.word_mul && existing.word_mul >= mul) {
    return;
  }
  map.set(key, {x, y, word_mul: mul, tags});
};

const setLetter = (map: BonusMap, x: number, y: number, mul: number, tags?: string[]) => {
  const key = keyFor(x, y);
  const existing = map.get(key);
  if (existing?.word_mul) {
    return;
  }
  if (existing?.letter_mul && existing.letter_mul >= mul) {
    return;
  }
  map.set(key, {x, y, letter_mul: mul, tags});
};

const buildBaseline = (width: number, height: number): BonusMap => {
  const map: BonusMap = new Map();
  const centerX = (width - 1) / 2;
  const centerY = (height - 1) / 2;

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const edgeDist = Math.min(x, width - 1 - x, y, height - 1 - y);
      if (edgeDist === 0) {
        setWord(map, x, y, 3);
        continue;
      }
      if (edgeDist === 1) {
        setWord(map, x, y, 2);
        continue;
      }

      const manhattanCenter = Math.abs(x - centerX) + Math.abs(y - centerY);
      if (manhattanCenter === 0) {
        continue;
      }

      if (manhattanCenter % 4 === 0) {
        setLetter(map, x, y, 3);
      } else if ((x + y) % 3 === 0) {
        setLetter(map, x, y, 2);
      }
    }
  }

  const midX = Math.round(centerX);
  const midY = Math.round(centerY);
  if (midX >= 0 && midX < width && midY >= 0 && midY < height) {
    setWord(map, midX, midY, 2, ['center']);
  }

  return map;
};

const applyHexAccents = (map: BonusMap, width: number, height: number) => {
  const centerX = (width - 1) / 2;
  const centerY = (height - 1) / 2;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const axialQ = x - centerX;
      const axialR = y - centerY;
      const axialS = -axialQ - axialR;
      const radius = Math.max(Math.abs(axialQ), Math.abs(axialR), Math.abs(axialS));
      if (radius === 0) continue;
      if (radius === 2) {
        setLetter(map, x, y, 3, ['hex']);
      } else if (radius === 3) {
        setLetter(map, x, y, 2, ['hex']);
      }
    }
  }
};

const applyTriangleAccents = (map: BonusMap, width: number, height: number) => {
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const diag = x - y;
      if (Math.abs(diag) === 0) {
        setLetter(map, x, y, 3, ['diag']);
      } else if (Math.abs(diag) === 1) {
        setLetter(map, x, y, 2, ['diag']);
      }
    }
  }
};

const applyRingAccents = (map: BonusMap, width: number, height: number) => {
  const centerX = (width - 1) / 2;
  const centerY = (height - 1) / 2;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const dist = Math.max(Math.abs(x - centerX), Math.abs(y - centerY));
      if (dist === 2) {
        setWord(map, x, y, 3, ['ring']);
      } else if (dist === 3) {
        setWord(map, x, y, 2, ['ring']);
      }
    }
  }
};

const buildBonusMap = (kind: BonusKind, width: number, height: number): BonusMap => {
  const map = buildBaseline(width, height);
  if (kind === 'hex') {
    applyHexAccents(map, width, height);
  } else if (kind === 'triangle') {
    applyTriangleAccents(map, width, height);
  } else if (kind === 'ring') {
    applyRingAccents(map, width, height);
  }
  return map;
};

const mapToArray = (map: BonusMap, width: number, height: number): BonusCell[] => {
  return Array.from(map.values()).filter(cell => cell.x >= 0 && cell.x < width && cell.y >= 0 && cell.y < height);
};

const presetToKind = (
  preset: BonusPreset,
  shape: BoardShape,
  adjacency: AdjacencyMode,
): BonusKind | 'none' => {
  if (preset === 'none') return 'none';
  if (preset === 'classic') return 'classic';
  if (preset === 'hex') return 'hex';
  if (preset === 'triangle') return 'triangle';
  if (preset === 'ring') return 'ring';
  // auto
  if (adjacency === 'hex' || shape === 'diamond') return 'hex';
  if (shape === 'triangle') return 'triangle';
  if (shape === 'ring') return 'ring';
  return 'classic';
};

export function resolveBonusPreset(
  preset: BonusPreset,
  shape: BoardShape,
  width: number,
  height: number,
  adjacency: AdjacencyMode,
): BonusCell[] {
  const kind = presetToKind(preset, shape, adjacency);
  if (kind === 'none') {
    return [];
  }
  const map = buildBonusMap(kind, width, height);
  return mapToArray(map, width, height);
}
