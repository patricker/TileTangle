import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import {classicTilesets} from './demoUtils';

const fallbackDictionaryWords = [
  'AA', 'AB', 'AD', 'AE', 'AGO', 'ALE', 'ARK', 'ART', 'BAD', 'BAG', 'BAL', 'BAR',
  'BAT', 'BEE', 'BOG', 'CAB', 'CAD', 'CAN', 'CAR', 'CAT', 'COD', 'COG', 'COT',
  'DOG', 'DOT', 'EAR', 'EEL', 'ELF', 'ERA', 'FAN', 'FAR', 'FAST', 'FEED', 'FILE',
  'FINE', 'FIR', 'FOG', 'FOOT', 'GAME', 'GATE', 'GO', 'HAT', 'HERO', 'ICE', 'INK',
  'JAR', 'JIG', 'KID', 'KIN', 'LAP', 'LID', 'MAP', 'NAP', 'OAR', 'OAT', 'PAD',
  'PAN', 'PINE', 'PLAY', 'QUIZ', 'READ', 'READS', 'ROAD', 'ROPE', 'RUN', 'SAGE',
  'SAND', 'SEA', 'STACK', 'STACKS', 'STONE', 'SUN', 'TAR', 'TIDE', 'TREE', 'TREES',
  'USE', 'VAST', 'WARP', 'WAVE', 'WORD', 'WORDS', 'WORK', 'YARD', 'ZEN'
];
const fallbackDictionaryText = fallbackDictionaryWords.join('\n');

const clamp = (value: number, min: number, max: number): number => {
  if (!Number.isFinite(value)) return min;
  return Math.min(Math.max(value, min), max);
};

const formatTileCounts = (counts: Record<string, number>): string => {
  return Object.entries(counts)
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([id, count]) => `${id}:${count}`)
    .join('\n');
};

const parseTileCounts = (text: string, fallback: Record<string, number>) => {
  const trimmed = text.trim();
  if (!trimmed) {
    return {map: fallback, error: 'Tile pool cannot be empty.'};
  }
  const map: Record<string, number> = {};
  const tokens = trimmed.split(/[,\n]+/);
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

const formatTileScores = (scores: Record<string, number>): string => {
  return Object.entries(scores)
    .sort((a, b) => a[0].localeCompare(b[0]))
    .map(([id, score]) => `${id}:${score}`)
    .join('\n');
};

const parseTileScores = (text: string, fallback: Record<string, number>) => {
  const map: Record<string, number> = {...fallback};
  const trimmed = text.trim();
  if (!trimmed) {
    return {map, error: 'Tile scores cannot be empty.'};
  }
  const tokens = trimmed.split(/[,\n]+/);
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

const buildShapeMask = (width: number, height: number, shape: BoardShape): Set<string> => {
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
    }
  }
  // Safety: if mask ended up empty (e.g. huge radius trimming), fall back to full rect
  if (mask.size === 0) {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        mask.add(`${x},${y}`);
      }
    }
  }
  return mask;
};

type BoardJson = {width: number; height: number; rows: string[][]};
type Placement = {x: number; y: number; kind_id: string; mark?: string | null};
type GeneratedMove = {
  word: string;
  score: number;
  total?: number;
  placements: Placement[];
};

type AiSuggestion = {
  difficulty: string;
  word: string;
  score: number;
  total: number;
  rackLeave: number;
  boardEquity: number;
  endgamePenalty: number;
  placements: Placement[];
};

type PlaygroundInitialConfig = {
  tileset?: {tile_kinds: {id: string; symbol: string; score: number; is_blank?: boolean; aliases?: string[]}[]};
  tile_counts?: Record<string, number>;
  rack_size?: number;
  board_layout?: Record<string, unknown>;
  ruleset_id?: string;
  dictionary_id?: string;
  rng_seed?: number;
};

type PlaygroundInitial = {
  useWorker?: boolean;
  useDict?: boolean;
  dictEngine?: 'fst' | 'set' | 'dawg' | 'gaddag';
  useAnagram?: boolean;
  useHex?: boolean;
  useDiag?: boolean;
  use3D?: boolean;
  depth?: number;
  rtl?: boolean;
  stackOn?: boolean;
  stackScoring?: 'top' | 'sum';
  forbidSame?: boolean;
  cpuDifficulty?: 'off' | 'easy' | 'medium' | 'hard';
  config?: PlaygroundInitialConfig;
  rack?: string[];
};

type PlaygroundProps = {
  initial?: PlaygroundInitial;
};

type ControlSectionProps = {
  title: string;
  children: React.ReactNode;
};

type PlayerSummary = {
  index: number;
  score: number;
  rack: string[];
};

type BagSummary = {
  counts: Record<string, number>;
  total: number;
};

type BoardShape = 'rect' | 'diamond' | 'cross';

type SetupState = {
  width: number;
  height: number;
  depth: number;
  rackSize: number;
  tileCountsText: string;
  tileScoresText: string;
  shape: BoardShape;
};

function ControlSection({title, children}: ControlSectionProps): JSX.Element {
  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 4}}>
      <div style={{fontWeight: 600, fontSize: 13}}>{title}</div>
      <div style={{display: 'flex', flexWrap: 'wrap', gap: 12, alignItems: 'center'}}>{children}</div>
    </div>
  );
}

export default function Playground({initial}: PlaygroundProps = {}): JSX.Element {
  const initialConfig = initial?.config;
  const initialLayout = (initialConfig?.board_layout ?? {}) as Record<string, any>;
  const initialWidth = typeof initialLayout.width === 'number' && initialLayout.width > 0 ? initialLayout.width : 9;
  const initialHeight = typeof initialLayout.height === 'number' && initialLayout.height > 0 ? initialLayout.height : 9;
  const initialDepth = typeof initialLayout.depth === 'number' && initialLayout.depth > 0 ? initialLayout.depth : (initial?.depth ?? 1);
  const initialRackSize = initialConfig?.rack_size ?? 7;

  const {colorMode} = useColorMode();

  const palette = useMemo(() => {
    if (colorMode === 'dark') {
      return {
        panelBg: 'rgba(26, 32, 44, 0.85)',
        panelBorder: 'rgba(148, 163, 184, 0.35)',
        playerActiveBg: 'rgba(56, 189, 248, 0.25)',
        playerBg: 'rgba(30, 41, 59, 0.35)',
        boardCellBg: 'rgba(15, 23, 42, 0.9)',
        boardCellBorder: 'rgba(148, 163, 184, 0.35)',
        boardCellHighlight: 'rgba(56, 189, 248, 0.35)',
        rackTileBg: 'rgba(30, 41, 59, 0.85)',
        rackTileBorder: 'rgba(148, 163, 184, 0.4)',
        rackTileHighlight: 'rgba(56, 189, 248, 0.35)',
        warningBg: 'rgba(234, 179, 8, 0.12)',
        warningBorder: 'rgba(250, 204, 21, 0.45)',
        errorBg: 'rgba(248, 113, 113, 0.12)',
        errorBorder: 'rgba(248, 113, 113, 0.55)',
        infoBg: 'rgba(59, 130, 246, 0.18)',
        infoBorder: 'rgba(96, 165, 250, 0.5)',
        accentBorder: 'rgba(59, 130, 246, 0.65)',
        textSubtle: 'rgba(226, 232, 240, 0.75)',
      } as const;
    }
    return {
      panelBg: '#f9fafb',
      panelBorder: 'var(--ifm-color-emphasis-200)',
      playerActiveBg: '#e0f2fe',
      playerBg: '#f9fafb',
      boardCellBg: '#ffffff',
      boardCellBorder: '#d1d5db',
      boardCellHighlight: '#e0f2fe',
      rackTileBg: '#f9fafb',
      rackTileBorder: '#d1d5db',
      rackTileHighlight: '#bae6fd',
      warningBg: '#fef3c7',
      warningBorder: '#facc15',
      errorBg: '#fee2e2',
      errorBorder: '#f87171',
      infoBg: '#dbeafe',
      infoBorder: '#60a5fa',
      accentBorder: '#38bdf8',
      textSubtle: 'rgba(15, 23, 42, 0.65)',
    } as const;
  }, [colorMode]);

  const classic = useMemo(() => classicTilesets(), []);
  const defaultTileKinds = useMemo(() => classic.tile_kinds, [classic]);
  const defaultTileCounts = useMemo(() => classic.tile_counts, [classic]);
  const defaultTileScores = useMemo(() => {
    const map: Record<string, number> = {};
    for (const kind of defaultTileKinds) {
      map[kind.id] = kind.score;
    }
    const initialKinds = initialConfig?.tileset?.tile_kinds;
    if (Array.isArray(initialKinds)) {
      for (const kind of initialKinds) {
        if (kind && typeof kind.id === 'string' && typeof kind.score === 'number') {
          map[kind.id] = kind.score;
        }
      }
    }
    return map;
  }, [defaultTileKinds, initialConfig?.tileset?.tile_kinds]);
  const initialTileCountsText = useMemo(
    () => formatTileCounts(initialConfig?.tile_counts ?? defaultTileCounts),
    [initialConfig?.tile_counts, defaultTileCounts],
  );
  const initialTileScoresText = useMemo(
    () => formatTileScores(defaultTileScores),
    [defaultTileScores],
  );

  const [useWorker, setUseWorker] = useState(initial?.useWorker ?? false);
  const [useDict, setUseDict] = useState(initial?.useDict ?? true);
  const [dictEngine, setDictEngine] = useState<'fst' | 'set' | 'dawg' | 'gaddag'>(initial?.dictEngine ?? 'fst');
  const [useAnagram, setUseAnagram] = useState(initial?.useAnagram ?? false);
  const [useHex, setUseHex] = useState(initial?.useHex ?? false);
  const [useDiag, setUseDiag] = useState(initial?.useDiag ?? false);
  const [use3D, setUse3D] = useState(initial?.use3D ?? false);
  const [rtl, setRtl] = useState(initial?.rtl ?? false);
  const [stackOn, setStackOn] = useState(initial?.stackOn ?? false);
  const [stackScoring, setStackScoring] = useState<'top' | 'sum'>(initial?.stackScoring ?? 'top');
  const [forbidSame, setForbidSame] = useState(initial?.forbidSame ?? true);
  const [z, setZ] = useState(0);
  const rackOverrideRef = useRef<string[] | undefined>(initial?.rack);
  const workerRef = useRef<Worker | null>(null);
  const workerRequestId = useRef(0);
  const wasmModuleRef = useRef<any | null>(null);
  const wasmModulePromiseRef = useRef<Promise<any> | null>(null);

  const ensureWasmModule = useCallback(async () => {
    if (wasmModuleRef.current) {
      return wasmModuleRef.current;
    }
    if (!wasmModulePromiseRef.current) {
      wasmModulePromiseRef.current = (async () => {
        const mod = await import('/wasm/engine/pkg/tiletangle_wasm.js');
        await mod.default();
        wasmModuleRef.current = mod;
        return mod;
      })();
    }
    return wasmModulePromiseRef.current;
  }, []);

  const terminateWorker = useCallback(() => {
    if (workerRef.current) {
      workerRef.current.terminate();
      workerRef.current = null;
    }
  }, []);

  const ensureWorker = useCallback(() => {
    if (typeof window === 'undefined') {
      return null;
    }
    if (!workerRef.current) {
      workerRef.current = new Worker('/wasm/engine/worker.js', {type: 'module'});
    }
    return workerRef.current;
  }, []);

  const callWorker = useCallback((action: string, payload?: any): Promise<any> => {
    const worker = workerRef.current;
    if (!worker) {
      return Promise.reject(new Error('Worker not ready'));
    }
    return new Promise((resolve, reject) => {
      const id = `req_${Date.now()}_${(++workerRequestId.current).toString(36)}`;
      const listener = (event: MessageEvent) => {
        const message = event.data as any;
        if (message?.id === id) {
          worker.removeEventListener('message', listener);
          if (message.ok) {
            resolve(message);
          } else {
            reject(new Error(message.error ?? 'Worker error'));
          }
        }
      };
      worker.addEventListener('message', listener);
      worker.postMessage({id, action, payload});
    });
  }, []);

  const [appliedSettings, setAppliedSettings] = useState<SetupState>({
    width: initialWidth,
    height: initialHeight,
    depth: initialDepth,
    rackSize: initialRackSize,
    tileCountsText: initialTileCountsText,
    tileScoresText: initialTileScoresText,
    shape: 'rect',
  });
  const [draftSettings, setDraftSettings] = useState<SetupState>(appliedSettings);
  const [tileCountsError, setTileCountsError] = useState<string | null>(null);
  const [tileScoresError, setTileScoresError] = useState<string | null>(null);
  const [newTileId, setNewTileId] = useState('');
  const [newTileCount, setNewTileCount] = useState(1);
  const [newTileScore, setNewTileScore] = useState(1);

  const draftCountsParsed = useMemo(
    () => parseTileCounts(draftSettings.tileCountsText, defaultTileCounts),
    [draftSettings.tileCountsText, defaultTileCounts],
  );
  const draftScoresParsed = useMemo(
    () => parseTileScores(draftSettings.tileScoresText, defaultTileScores),
    [draftSettings.tileScoresText, defaultTileScores],
  );
  const draftTileRows = useMemo(() => {
    const ids = new Set<string>([
      ...Object.keys(draftCountsParsed.map),
      ...Object.keys(draftScoresParsed.map),
    ]);
    return Array.from(ids)
      .sort((a, b) => a.localeCompare(b))
      .map(id => ({
        id,
        count: draftCountsParsed.map[id] ?? 0,
        score: draftScoresParsed.map[id] ?? 0,
      }));
  }, [draftCountsParsed, draftScoresParsed]);

  const updateTileEntry = useCallback(
    (id: string, updates: {count?: number; score?: number}) => {
      const normalized = id.toUpperCase();
      const counts = {...draftCountsParsed.map};
      const scores = {...draftScoresParsed.map};

      if (updates.count != null) {
        const nextCount = Number.isFinite(updates.count) ? Math.max(0, Math.trunc(updates.count)) : 0;
        if (nextCount <= 0) {
          delete counts[normalized];
          delete scores[normalized];
        } else {
          counts[normalized] = nextCount;
        }
      }

      if (updates.score != null) {
        const nextScore = Number.isFinite(updates.score) ? Math.trunc(updates.score) : 0;
        scores[normalized] = nextScore;
      }

      if (Object.keys(counts).length === 0) {
        setTileCountsError('Tile pool must include at least one tile.');
        return;
      }

      const countsText = formatTileCounts(counts);
      const scoresText = formatTileScores(scores);
      setDraftSettings(prev => ({...prev, tileCountsText: countsText, tileScoresText: scoresText}));
      const countsValidation = parseTileCounts(countsText, defaultTileCounts);
      setTileCountsError(countsValidation.error);
      const scoresValidation = parseTileScores(scoresText, defaultTileScores);
      setTileScoresError(scoresValidation.error);
    },
    [draftCountsParsed, draftScoresParsed, defaultTileCounts, defaultTileScores],
  );

  const removeTile = useCallback(
    (id: string) => {
      const normalized = id.toUpperCase();
      const counts = {...draftCountsParsed.map};
      const scores = {...draftScoresParsed.map};
      delete counts[normalized];
      delete scores[normalized];
      if (Object.keys(counts).length === 0) {
        setTileCountsError('Tile pool must include at least one tile.');
        return;
      }
      const countsText = formatTileCounts(counts);
      const scoresText = formatTileScores(scores);
      setDraftSettings(prev => ({...prev, tileCountsText: countsText, tileScoresText: scoresText}));
      const countsValidation = parseTileCounts(countsText, defaultTileCounts);
      setTileCountsError(countsValidation.error);
      const scoresValidation = parseTileScores(scoresText, defaultTileScores);
      setTileScoresError(scoresValidation.error);
    },
    [draftCountsParsed, draftScoresParsed, defaultTileCounts, defaultTileScores],
  );

  const handleAddTile = useCallback(() => {
    const normalized = newTileId.trim().toUpperCase();
    if (!normalized) return;
    const safeCount = Number.isFinite(newTileCount) ? Math.max(1, Math.trunc(newTileCount)) : 1;
    const safeScore = Number.isFinite(newTileScore) ? Math.trunc(newTileScore) : 0;
    updateTileEntry(normalized, {count: safeCount, score: safeScore});
    setNewTileId('');
    setNewTileCount(1);
    setNewTileScore(1);
  }, [newTileCount, newTileId, newTileScore, updateTileEntry]);

  const handleResetTiles = useCallback(() => {
    setDraftSettings(prev => ({
      ...prev,
      tileCountsText: initialTileCountsText,
      tileScoresText: initialTileScoresText,
    }));
    setTileCountsError(null);
    setTileScoresError(null);
    setNewTileId('');
    setNewTileCount(1);
    setNewTileScore(1);
  }, [initialTileCountsText, initialTileScoresText]);

  const appliedTileCountsResult = useMemo(
    () => parseTileCounts(appliedSettings.tileCountsText, defaultTileCounts),
    [appliedSettings.tileCountsText, defaultTileCounts],
  );
  const appliedTileCounts = appliedTileCountsResult.map;
  const appliedTileScoresResult = useMemo(
    () => parseTileScores(appliedSettings.tileScoresText, defaultTileScores),
    [appliedSettings.tileScoresText, defaultTileScores],
  );
  const appliedTileScores = appliedTileScoresResult.map;

  const dynamicTileKinds = useMemo(() => {
    const kinds = defaultTileKinds.map(kind => ({
      ...kind,
      score: appliedTileScores[kind.id] ?? kind.score ?? 0,
    }));
    const seen = new Set(kinds.map(k => k.id));
    const ensureKind = (id: string) => {
      if (seen.has(id)) return;
      const score = appliedTileScores[id] ?? 1;
      kinds.push({id, symbol: id, score, is_blank: false, aliases: []});
      seen.add(id);
    };
    for (const id of Object.keys(appliedTileCounts)) ensureKind(id);
    for (const id of Object.keys(appliedTileScores)) ensureKind(id);
    return kinds;
  }, [defaultTileKinds, appliedTileCounts, appliedTileScores]);

  const tileMetaMap = useMemo(() => {
    const map = new Map<string, {symbol: string; score: number}>();
    for (const kind of dynamicTileKinds) {
      const symbol = typeof kind.symbol === 'string' && kind.symbol.length > 0 ? kind.symbol : kind.id;
      const score = typeof kind.score === 'number' ? kind.score : appliedTileScores[kind.id] ?? 0;
      map.set(kind.id, {symbol, score});
      map.set(kind.id.toUpperCase(), {symbol, score});
    }
    return map;
  }, [dynamicTileKinds, appliedTileScores]);

  const getTileMeta = useCallback(
    (id: string) => tileMetaMap.get(id) ?? tileMetaMap.get(id.toUpperCase()),
    [tileMetaMap],
  );

  const activeMask = useMemo(() => {
    if (use3D) return null;
    return buildShapeMask(appliedSettings.width, appliedSettings.height, appliedSettings.shape);
  }, [use3D, appliedSettings.width, appliedSettings.height, appliedSettings.shape]);

  const isCellActive = useCallback(
    (x: number, y: number) => {
      if (!activeMask) return true;
      return activeMask.has(`${x},${y}`);
    },
    [activeMask],
  );

  const [ready, setReady] = useState(false);
  const [board, setBoard] = useState<BoardJson | null>(null);
  const [game, setGame] = useState<any>(null);
  const [pending, setPending] = useState<Placement[]>([]);
  const [showMoves, setShowMoves] = useState(false);
  const [legalMoves, setLegalMoves] = useState<GeneratedMove[]>([]);
  const [activeMoveIndex, setActiveMoveIndex] = useState<number | null>(null);
  const [loadingMoves, setLoadingMoves] = useState(false);
  const [rack, setRack] = useState<string[]>(rackOverrideRef.current ?? []);

  const rackCountByKind = useMemo(() => {
    const map = new Map<string, number>();
    for (const kind of rack) {
      map.set(kind, (map.get(kind) ?? 0) + 1);
    }
    return map;
  }, [rack]);

  const availableRackTiles = useMemo(() => {
    const remaining = new Map<string, number>();
    for (const placement of pending) {
      const key = placement.kind_id;
      remaining.set(key, (remaining.get(key) ?? 0) + 1);
    }
    const arr: {id: string; index: number}[] = [];
    rack.forEach((id, index) => {
      const pendingCount = remaining.get(id) ?? 0;
      if (pendingCount > 0) {
        remaining.set(id, pendingCount - 1);
      } else {
        arr.push({id, index});
      }
    });
    return arr;
  }, [rack, pending]);

  const [players, setPlayers] = useState<PlayerSummary[]>([]);
  const [activePlayer, setActivePlayer] = useState(0);
  const [turnNumber, setTurnNumber] = useState(0);
  const [bagSummary, setBagSummary] = useState<BagSummary>({counts: {}, total: 0});
  const [cpuDifficulty, setCpuDifficulty] = useState<'off' | 'easy' | 'medium' | 'hard'>(initial?.cpuDifficulty ?? 'off');
  const [cpuThinking, setCpuThinking] = useState(false);
  const [cpuSuggestion, setCpuSuggestion] = useState<AiSuggestion | null>(null);
  const [lastCpu, setLastCpu] = useState<AiSuggestion | null>(null);
  const [cpuError, setCpuError] = useState<string | null>(null);
  const [dictReady, setDictReady] = useState(!useDict);
  const [dictError, setDictError] = useState<string | null>(null);
  const [dictLoading, setDictLoading] = useState(false);
  const [snapshotText, setSnapshotText] = useState('');
  const [eventLogText, setEventLogText] = useState('');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [infoMessage, setInfoMessage] = useState<string | null>(null);
  const [anagramIndex, setAnagramIndex] = useState<Map<string, string> | null>(null);

  const effectiveDepth = use3D ? appliedSettings.depth : 1;

  useEffect(() => {
    setZ(prev => Math.min(prev, Math.max(0, effectiveDepth - 1)));
  }, [effectiveDepth]);

  useEffect(() => {
    if (!use3D) {
      setInfoMessage(null);
    }
  }, [use3D]);

  useEffect(() => {
    let cancelled = false;

    async function buildIndex() {
      if (!useAnagram || !useDict) {
        setAnagramIndex(null);
        return;
      }
      try {
        const resp = await fetch('/dictionaries/TWL06.txt');
        if (!resp.ok) {
          setAnagramIndex(null);
          return;
        }
        const txt = await resp.text();
        const maxLen = appliedSettings.rackSize;
        const map = new Map<string, string>();
        for (const raw of txt.split(/\r?\n/)) {
          const word = raw.trim();
          if (!word || word.startsWith('#') || word.length > maxLen) continue;
          const sig = word.toUpperCase().split('').sort().join('');
          if (!map.has(sig)) map.set(sig, word.toUpperCase());
        }
        if (!cancelled) setAnagramIndex(map);
      } catch {
        if (!cancelled) setAnagramIndex(null);
      }
    }

    buildIndex();
    return () => {
      cancelled = true;
    };
  }, [useAnagram, useDict, appliedSettings.rackSize]);

  const cfg = useMemo(() => {
    const width = clamp(appliedSettings.width, 2, 30);
    const height = clamp(appliedSettings.height, 2, 30);
    const layoutDepth = clamp(appliedSettings.depth, 1, 12);

    const shapeMask = buildShapeMask(width, height, appliedSettings.shape);
    const shapeHasMask = appliedSettings.shape !== 'rect';
    const isActive = (x: number, y: number) => {
      if (shapeHasMask && !shapeMask.has(`${x},${y}`)) {
        return false;
      }
      return true;
    };

    let boardLayout: Record<string, unknown>;
    if (use3D) {
      boardLayout = {type: '3d', width, height, depth: layoutDepth};
    } else if (useDiag) {
      const nodes: {x: number; y: number}[] = [];
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (isActive(x, y)) nodes.push({x, y});
        }
      }
      const index = (x: number, y: number) => y * width + x;
      const edges: {a: number; b: number; dir: string}[] = [];
      const addEdge = (x1: number, y1: number, x2: number, y2: number, dir: string) => {
        if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
        if (!isActive(x1, y1) || !isActive(x2, y2)) return;
        edges.push({a: index(x1, y1), b: index(x2, y2), dir});
      };
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (!isActive(x, y)) continue;
          addEdge(x, y, x + 1, y, 'E');
          addEdge(x, y, x - 1, y, 'W');
          addEdge(x, y, x, y - 1, 'N');
          addEdge(x, y, x, y + 1, 'S');
          addEdge(x, y, x + 1, y - 1, 'NE');
          addEdge(x, y, x - 1, y - 1, 'NW');
          addEdge(x, y, x + 1, y + 1, 'SE');
          addEdge(x, y, x - 1, y + 1, 'SW');
        }
      }
      boardLayout = {width, height, type: 'graph', nodes, edges};
    } else if (useHex) {
      const nodes: {x: number; y: number}[] = [];
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (isActive(x, y)) nodes.push({x, y});
        }
      }
      const index = (x: number, y: number) => y * width + x;
      const edges: {a: number; b: number; dir: string}[] = [];
      const addEdge = (x1: number, y1: number, x2: number, y2: number, dir: string) => {
        if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
        if (!isActive(x1, y1) || !isActive(x2, y2)) return;
        edges.push({a: index(x1, y1), b: index(x2, y2), dir});
      };
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (!isActive(x, y)) continue;
          const even = y % 2 === 0;
          const eastShift = even ? 0 : 1;
          const westShift = even ? -1 : 0;
          addEdge(x, y, x + 1, y, 'E');
          addEdge(x, y, x - 1, y, 'W');
          addEdge(x, y, x + eastShift, y - 1, 'NE');
          addEdge(x, y, x + westShift, y - 1, 'NW');
          addEdge(x, y, x + eastShift, y + 1, 'SE');
          addEdge(x, y, x + westShift, y + 1, 'SW');
        }
      }
      boardLayout = {width, height, type: 'graph', nodes, edges};
    } else if (shapeHasMask) {
      const nodes: {x: number; y: number}[] = [];
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (isActive(x, y)) nodes.push({x, y});
        }
      }
      const index = (x: number, y: number) => y * width + x;
      const edges: {a: number; b: number; dir: string}[] = [];
      const addEdge = (x1: number, y1: number, x2: number, y2: number, dir: string) => {
        if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
        if (!isActive(x1, y1) || !isActive(x2, y2)) return;
        edges.push({a: index(x1, y1), b: index(x2, y2), dir});
      };
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (!isActive(x, y)) continue;
          addEdge(x, y, x + 1, y, 'E');
          addEdge(x, y, x - 1, y, 'W');
          addEdge(x, y, x, y - 1, 'N');
          addEdge(x, y, x, y + 1, 'S');
        }
      }
      boardLayout = {width, height, type: 'graph', nodes, edges};
    } else {
      boardLayout = {width, height};
    }

    const tileCountsRecord: Record<string, number> = {};
    for (const [id, count] of Object.entries(appliedTileCounts)) {
      tileCountsRecord[id] = count;
    }

    return {
      tileset: {tile_kinds: dynamicTileKinds},
      rack_size: clamp(appliedSettings.rackSize, 1, 14),
      board_layout: boardLayout,
      ruleset_id: initialConfig?.ruleset_id ?? 'cross',
      dictionary_id: initialConfig?.dictionary_id ?? 'en',
      rng_seed: initialConfig?.rng_seed ?? 1,
      tile_counts: tileCountsRecord,
      free_word_mode: !useDict,
    } as any;
  }, [
    appliedSettings.width,
    appliedSettings.height,
    appliedSettings.depth,
    appliedSettings.rackSize,
    appliedSettings.shape,
    appliedTileCounts,
    dynamicTileKinds,
    use3D,
    useDiag,
    useHex,
    useDict,
    initialConfig?.ruleset_id,
    initialConfig?.dictionary_id,
    initialConfig?.rng_seed,
  ]);

  const [dictMessagesVersion, setDictMessagesVersion] = useState(0);

  useEffect(() => {
    let cancelled = false;

    const loadDictionary = async (handlers: {
      applyText: (text: string) => Promise<void> | void;
      applyFst?: (bytes: Uint8Array) => Promise<void> | void;
      preferFst: boolean;
    }): Promise<boolean> => {
      const {applyText, applyFst, preferFst} = handlers;

      if (preferFst && applyFst) {
        try {
          const resp = await fetch('/dictionaries/TWL06.fst');
          if (resp.ok) {
            const buf = new Uint8Array(await resp.arrayBuffer());
            await applyFst(buf);
            return true;
          }
          console.warn('TWL06.fst fetch returned status', resp.status);
        } catch (err) {
          console.warn('FST dictionary fetch failed', err);
        }
      }

      const textSources = ['/dictionaries/TWL06.txt', '/dictionaries/demo.txt'];
      for (const url of textSources) {
        try {
          const resp = await fetch(url);
          if (!resp.ok) {
            console.warn(`Dictionary fetch from ${url} returned status`, resp.status);
            continue;
          }
          const txt = await resp.text();
          await applyText(txt);
          return true;
        } catch (err) {
          console.warn(`Dictionary fetch failed from ${url}`, err);
        }
      }

      try {
        await applyText(fallbackDictionaryText);
        return true;
      } catch (err) {
        console.warn('Fallback dictionary load failed', err);
      }
      return false;
    };

    const initialise = async () => {
      setReady(false);
      setPending([]);
      setShowMoves(false);
      setLegalMoves([]);
      setActiveMoveIndex(null);
      setPlayers([]);
      setBagSummary({counts: {}, total: 0});
      setCpuError(null);
      setErrorMessage(null);
      setInfoMessage(null);
      setDictLoading(useDict);
      setDictReady(!useDict);
      setDictError(null);

      try {
        if (useWorker) {
          const worker = ensureWorker();
          if (!worker) return;
          const call = callWorker;
          const cfg2 = {...cfg, free_word_mode: !useDict};
          await call('new_game', {config: cfg2, players: 2});
          if (cancelled) return;
          await call('set_reading_direction', {rtl});
          await call('set_stacking', {
            enabled: stackOn,
            max_height: 7,
            forbid_same: forbidSame,
            scoring: stackScoring,
          });
          await call('set_free_word_mode', {on: !useDict});

          let dictionaryLoaded = !useDict;
          if (useDict) {
            dictionaryLoaded = await loadDictionary({
              preferFst: dictEngine === 'fst',
              applyFst: async bytes => {
                await call('set_dictionary_from_fst_bytes', {bytes, case_fold: true});
              },
              applyText: async text => {
                await call('set_dictionary_engine', {text, engine: dictEngine, case_fold: true});
              },
            });
          }

          if (!dictionaryLoaded) {
            await call('set_free_word_mode', {on: true});
            if (!cancelled) {
              setDictReady(false);
              setDictError('Dictionary failed to load. Free-word mode enabled.');
              if (useDict) setUseDict(false);
            }
          } else if (!cancelled) {
            setDictReady(true);
            setDictError(null);
          }

          if (rackOverrideRef.current?.length) {
            await call('set_rack', {tiles: rackOverrideRef.current});
          }

          const [{board: boardStr}, snapshotResp] = await Promise.all([
            call('get_board'),
            call('snapshot_json'),
          ]);
          if (cancelled) return;
          applySnapshot(JSON.parse(boardStr as string) as BoardJson, snapshotResp.snapshot as string);
          setGame({call});
          setReady(true);
        } else {
          terminateWorker();
          const mod = await ensureWasmModule();
          const cfg2 = {...cfg, free_word_mode: !useDict};
          const g = mod.new_game(JSON.stringify(cfg2), 2);
          mod.set_reading_direction(g, rtl);
          mod.set_stacking(g, stackOn, 7, forbidSame, stackScoring === 'sum');
          mod.set_free_word_mode(g, !useDict);

          let dictionaryLoaded = !useDict;
          if (useDict) {
            dictionaryLoaded = await loadDictionary({
              preferFst: dictEngine === 'fst',
              applyFst: async bytes => {
                mod.set_dictionary_from_fst_bytes(g, bytes, true);
              },
              applyText: async text => {
                mod.set_dictionary_from_text_engine(g, text, dictEngine, true);
              },
            });
          }

          if (!dictionaryLoaded) {
            mod.set_free_word_mode(g, true);
            if (!cancelled) {
              setDictReady(false);
              setDictError('Dictionary failed to load. Free-word mode enabled.');
              if (useDict) setUseDict(false);
            }
          } else if (!cancelled) {
            setDictReady(true);
            setDictError(null);
          }

          if (rackOverrideRef.current?.length) {
            mod.set_rack(g, JSON.stringify(rackOverrideRef.current));
          }

          const boardStr = mod.get_board(g);
          const snapshotStr = mod.snapshot_state_json(g);
          if (cancelled) return;
          applySnapshot(JSON.parse(boardStr) as BoardJson, snapshotStr);
          setGame({mod, g});
          setReady(true);
        }
      } catch (err) {
        console.error('Initialise playground failed', err);
        if (!cancelled) {
          setDictError('Initialisation failed. Check console for details.');
          setDictReady(false);
        }
      } finally {
        if (!cancelled) {
          setDictLoading(false);
        }
      }
    };

    initialise();

    return () => {
      cancelled = true;
      terminateWorker();
    };
  }, [
    cfg,
    useWorker,
    useDict,
    rtl,
    stackOn,
    stackScoring,
    forbidSame,
    dictEngine,
    dictMessagesVersion,
    ensureWorker,
    callWorker,
    ensureWasmModule,
    terminateWorker,
  ]);

  const applySnapshot = useCallback((boardJson: BoardJson, snapshotJson: string) => {
    setBoard(boardJson);
    try {
      const state = JSON.parse(snapshotJson);
      const toMove: number = typeof state.to_move === 'number' ? state.to_move : 0;
      const playersRaw: any[] = Array.isArray(state.players) ? state.players : [];
      const summaries: PlayerSummary[] = playersRaw.map((p, idx) => ({
        index: idx,
        score: typeof p.score === 'number' ? p.score : 0,
        rack: Array.isArray(p.rack?.tiles) ? p.rack.tiles.map((t: any) => t.kind_id ?? '?') : [],
      }));
      const safeActive = summaries.length > 0 ? toMove % summaries.length : 0;
      setPlayers(summaries);
      setActivePlayer(safeActive);
      setTurnNumber(typeof state.turn_num === 'number' ? state.turn_num : 0);
      setRack(summaries[safeActive]?.rack ?? []);

      const bagList: any[] = Array.isArray(state.bag?.counts) ? state.bag.counts : [];
      const counts: Record<string, number> = {};
      let total = 0;
      for (const entry of bagList) {
        if (Array.isArray(entry) && entry.length >= 2) {
          const kind = entry[0];
          const count = entry[1];
          const id = kind?.id ?? kind?.symbol ?? '?';
          if (typeof count === 'number') {
            counts[id] = (counts[id] ?? 0) + count;
            total += count;
          }
        }
      }
      setBagSummary({counts, total});
    } catch (err) {
      console.error('Failed to parse snapshot', err);
    }
  }, []);

  const updateFromGame = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        const [{board: b}, snap] = await Promise.all([
          game.call('get_board'),
          game.call('snapshot_json'),
        ]);
        applySnapshot(JSON.parse(b as string) as BoardJson, snap.snapshot as string);
      } else {
        const boardStr = game.mod.get_board(game.g);
        const snapshotStr = game.mod.snapshot_state_json(game.g);
        applySnapshot(JSON.parse(boardStr) as BoardJson, snapshotStr);
      }
    } catch (err) {
      console.error('sync board failed', err);
    }
  }, [game, useWorker, applySnapshot]);

  const handleApplySettings = useCallback(() => {
    const width = clamp(draftSettings.width, 2, 30);
    const height = clamp(draftSettings.height, 2, 30);
    const depth = clamp(draftSettings.depth, 1, 12);
    const rackSize = clamp(draftSettings.rackSize, 1, 14);
    const countsParsed = parseTileCounts(draftSettings.tileCountsText, defaultTileCounts);
    if (countsParsed.error) {
      setTileCountsError(countsParsed.error);
      return;
    }
    const scoresParsed = parseTileScores(draftSettings.tileScoresText, defaultTileScores);
    if (scoresParsed.error) {
      setTileScoresError(scoresParsed.error);
      return;
    }
    setTileCountsError(null);
    setTileScoresError(null);
    setAppliedSettings({
      width,
      height,
      depth,
      rackSize,
      tileCountsText: formatTileCounts(countsParsed.map),
      tileScoresText: formatTileScores(scoresParsed.map),
      shape: draftSettings.shape,
    });
    setDictMessagesVersion(v => v + 1);
  }, [draftSettings, defaultTileCounts, defaultTileScores]);

  const dedupeMoves = (moves: GeneratedMove[]): GeneratedMove[] => {
    const map = new Map<string, GeneratedMove>();
    for (const mv of moves) {
      const placements = [...mv.placements]
        .map(p => ({x: p.x, y: p.y, kind: p.kind_id ?? '', mark: p.mark ?? ''}))
        .sort((a, b) => (a.y - b.y) || (a.x - b.x) || a.kind.localeCompare(b.kind) || a.mark.localeCompare(b.mark));
      const key = JSON.stringify({word: mv.word ?? '', score: mv.score ?? 0, total: mv.total ?? mv.score ?? 0, placements});
      if (!map.has(key)) {
        map.set(key, mv);
      }
    }
    return Array.from(map.values());
  };

  const fetchMoves = useCallback(async () => {
    if (!game) return;
    if (use3D) {
      setInfoMessage('Automatic move generation is not yet available for 3D boards. Switch to 2D to inspect suggestions.');
      setLegalMoves([]);
      setActiveMoveIndex(null);
      setShowMoves(true);
      return;
    }
    setInfoMessage(null);
    setLoadingMoves(true);
    try {
      let moves: GeneratedMove[] = [];
      const maxLen = appliedSettings.rackSize;
      if (useWorker) {
        const resp = await game.call('generate_moves', {max_len: maxLen, limit: 30});
        moves = JSON.parse(resp.moves as string) as GeneratedMove[];
      } else {
        const json = game.mod.generate_moves(game.g, maxLen, 30);
        moves = JSON.parse(json) as GeneratedMove[];
      }
      const deduped = dedupeMoves(moves);
      setLegalMoves(deduped);
      setActiveMoveIndex(deduped.length ? 0 : null);
      setShowMoves(true);
    } catch (err) {
      console.error('generate_moves failed', err);
      setErrorMessage(`Move generation failed: ${(err as Error).message ?? String(err)}`);
      setLegalMoves([]);
      setActiveMoveIndex(null);
      setShowMoves(true);
    } finally {
      setLoadingMoves(false);
    }
  }, [game, useWorker, use3D, appliedSettings.rackSize]);

  const commitMove = useCallback(async () => {
    if (!game || pending.length === 0) return;
    try {
      let placements = pending;
      if (useAnagram && useDict && anagramIndex && board) {
        const xs = new Set(placements.map(p => p.x));
        const ys = new Set(placements.map(p => p.y));
        const isRow = ys.size === 1;
        const isCol = xs.size === 1;
        if (isRow || isCol) {
          const letters = placements.map(p => p.kind_id.toUpperCase());
          const sig = letters.slice().sort().join('');
          const word = anagramIndex.get(sig);
          if (word) {
            const sorted = [...placements].sort((a, b) => (isRow ? a.x - b.x : a.y - b.y));
            placements = sorted.map((p, idx) => ({...p, kind_id: word[idx]}));
          }
        }
      }

      if (useWorker) {
        await game.call('play_move', {placements});
      } else {
        game.mod.play_move(game.g, JSON.stringify(placements));
      }
      await updateFromGame();
      setPending([]);
      setCpuSuggestion(null);
      setLastCpu(null);
      setShowMoves(false);
      setErrorMessage(null);
    } catch (err) {
      console.error(err);
      setErrorMessage(`Move failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, pending, useWorker, useAnagram, useDict, anagramIndex, board, updateFromGame]);

  const playGeneratedMove = useCallback(async (move: GeneratedMove) => {
    if (!game) return;
    try {
      const placements = move.placements.map(p => ({x: p.x, y: p.y, kind_id: p.kind_id, mark: p.mark ?? null}));
      if (useWorker) {
        await game.call('play_move', {placements});
      } else {
        game.mod.play_move(game.g, JSON.stringify(placements));
      }
      await updateFromGame();
      setPending([]);
      setCpuSuggestion(null);
      setShowMoves(false);
      setErrorMessage(null);
    } catch (err) {
      console.error('play_generated_move failed', err);
      setErrorMessage(`Auto-play failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, useWorker, updateFromGame]);

  const requestCpuHint = useCallback(async () => {
    if (!game || cpuDifficulty === 'off' || !dictReady || use3D) {
      setCpuSuggestion(null);
      return;
    }
    setCpuThinking(true);
    setCpuError(null);
    try {
      let bestJson: string;
      if (useWorker) {
        const resp = await game.call('best_move', {difficulty: cpuDifficulty, seed: BigInt(42)});
        bestJson = resp.best as string;
      } else {
        bestJson = game.mod.best_move(game.g, cpuDifficulty, BigInt(42));
      }
      const payload = JSON.parse(bestJson);
      const placements: Placement[] = (payload.placements || []).map((p: any) => ({
        x: p.x,
        y: p.y,
        kind_id: p.kind_id,
        mark: p.mark ?? null,
      }));
      const suggestion: AiSuggestion = {
        difficulty: cpuDifficulty,
        word: payload.word,
        score: payload.score,
        total: payload.total,
        rackLeave: payload.rack_leave,
        boardEquity: payload.board_equity,
        endgamePenalty: payload.endgame_penalty,
        placements,
      };
      setCpuSuggestion(suggestion);
      setLastCpu(suggestion);
    } catch (err) {
      console.warn('best_move failed', err);
      const message = err instanceof Error ? err.message : String(err);
      if (message.toLowerCase().includes('no moves available')) {
        setCpuError('CPU has no available moves for the current rack.');
      } else {
        setCpuError(`CPU hint failed: ${message}`);
      }
      setCpuSuggestion(null);
    } finally {
      setCpuThinking(false);
    }
  }, [game, useWorker, cpuDifficulty, dictReady, use3D]);

  const playCpuSuggestion = useCallback(async () => {
    if (!cpuSuggestion) return;
    await playGeneratedMove({
      word: cpuSuggestion.word,
      score: cpuSuggestion.score,
      total: cpuSuggestion.total,
      placements: cpuSuggestion.placements,
    });
    setLastCpu(cpuSuggestion);
    setCpuSuggestion(null);
  }, [cpuSuggestion, playGeneratedMove]);

  const exportSnapshot = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        const resp = await game.call('snapshot_json');
        setSnapshotText(String((resp as any)?.snapshot ?? ''));
      } else {
        const snap = game.mod.snapshot_state_json(game.g);
        setSnapshotText(snap);
      }
    } catch (err) {
      console.error('snapshot export failed', err);
      setErrorMessage(`Snapshot export failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, useWorker]);

  const importSnapshot = useCallback(async () => {
    if (!game || !snapshotText.trim()) return;
    try {
      if (useWorker) {
        await game.call('load_snapshot_json', {json: snapshotText});
      } else {
        game.mod.load_state_json(game.g, snapshotText);
      }
      await updateFromGame();
      setPending([]);
      setCpuSuggestion(null);
      setLastCpu(null);
      setShowMoves(false);
    } catch (err) {
      console.error('snapshot import failed', err);
      setErrorMessage(`Snapshot import failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, snapshotText, useWorker, updateFromGame]);

  const fetchEventLog = useCallback(async () => {
    if (!game) return;
    try {
      let payload: string;
      if (useWorker) {
        const resp = await game.call('event_log');
        payload = String((resp as any)?.log ?? '[]');
      } else {
        payload = game.mod.get_event_log(game.g);
      }
      try {
        const formatted = JSON.stringify(JSON.parse(payload), null, 2);
        setEventLogText(formatted);
      } catch {
        setEventLogText(payload);
      }
    } catch (err) {
      console.error('fetch event log failed', err);
      setErrorMessage(`Fetching event log failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, useWorker]);

  const undoMove = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        await game.call('undo');
      } else {
        game.mod.undo(game.g);
      }
      await updateFromGame();
    } catch (err) {
      console.error('undo failed', err);
      setErrorMessage(`Undo failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, useWorker, updateFromGame]);

  const redoMove = useCallback(async () => {
    if (!game) return;
    try {
      if (useWorker) {
        await game.call('redo');
      } else {
        game.mod.redo(game.g);
      }
      await updateFromGame();
    } catch (err) {
      console.error('redo failed', err);
      setErrorMessage(`Redo failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, useWorker, updateFromGame]);

  const handlePendingReset = useCallback(() => {
    setPending([]);
    setErrorMessage(null);
  }, []);

  const onDropCell = (x: number, y: number, ev: React.DragEvent<HTMLDivElement>) => {
    ev.preventDefault();
    const kindId = ev.dataTransfer.getData('text/plain');
    if (!kindId) return;
    setPending(prev => {
      if (prev.some(p => p.x === x && p.y === y)) return prev;
      if (!board) return prev;
      const layerHeight = use3D ? Math.floor(board.height / Math.max(1, effectiveDepth)) : board.height;
      const globalY = use3D ? y + z * layerHeight : y;
      if (!use3D && !isCellActive(x, globalY)) {
        return prev;
      }
      if (!stackOn && (board.rows[globalY]?.[x] || '').length > 0) return prev;
      const available = rackCountByKind.get(kindId) ?? 0;
      const used = prev.filter(p => p.kind_id === kindId).length;
      if (used >= available && available > 0) {
        return prev;
      }
      if (available === 0) {
        return prev;
      }
      return [...prev, {x, y: globalY, kind_id: kindId}];
    });
  };

  const onDragStartTile = (kindId: string, ev: React.DragEvent<HTMLDivElement>) => {
    ev.dataTransfer.setData('text/plain', kindId);
  };

  const cellDisplay = (x: number, y: number): string => {
    const placement = pending.find(p => p.x === x && p.y === (use3D ? y + z * Math.floor(board!.height / Math.max(1, effectiveDepth)) : y));
    if (placement) return placement.kind_id;
    if (use3D && board) {
      const layerHeight = Math.floor(board.height / Math.max(1, effectiveDepth));
      const idx = y + z * layerHeight;
      return board.rows[idx]?.[x] ?? '';
    }
    return board?.rows[y]?.[x] ?? '';
  };

  const highlightCells = useMemo(() => {
    if (activeMoveIndex === null) return new Set<string>();
    const mv = legalMoves[activeMoveIndex];
    if (!mv) return new Set<string>();
    const set = new Set<string>();
    for (const p of mv.placements) set.add(`${p.x},${p.y}`);
    return set;
  }, [activeMoveIndex, legalMoves]);

  const bagPreview = useMemo(() => {
    const entries = Object.entries(bagSummary.counts)
      .filter(([, count]) => count > 0)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6)
      .map(([id, count]) => `${id}:${count}`);
    return entries.join(', ');
  }, [bagSummary]);

  if (!ready || !board) {
    return <div>Loading WASM…</div>;
  }

  const layerHeight = use3D ? Math.floor(board.height / Math.max(1, effectiveDepth)) : board.height;

  return (
    <div style={{display: 'flex', flexDirection: 'column', gap: 16}}>
      {(errorMessage || dictError || cpuError) && (
        <div style={{display: 'flex', flexDirection: 'column', gap: 6}}>
          {errorMessage && (
            <div style={{background: palette.errorBg, border: `1px solid ${palette.errorBorder}`, borderRadius: 4, padding: 8}}>
              {errorMessage}
            </div>
          )}
          {dictError && (
            <div style={{background: palette.warningBg, border: `1px solid ${palette.warningBorder}`, borderRadius: 4, padding: 8}}>
              {dictError}
            </div>
          )}
          {cpuError && (
            <div style={{background: palette.warningBg, border: `1px solid ${palette.warningBorder}`, borderRadius: 4, padding: 8}}>
              {cpuError}
            </div>
          )}
        </div>
      )}
      {infoMessage && (
        <div style={{background: palette.infoBg, border: `1px solid ${palette.infoBorder}`, borderRadius: 4, padding: 8}}>
          {infoMessage}
        </div>
      )}

      <ControlSection title="Game Setup">
        <label>Width:
          <input
            type="number"
            min={2}
            max={30}
            value={draftSettings.width}
            onChange={e => setDraftSettings(prev => ({...prev, width: Number(e.target.value)}))}
            style={{width: 70}}
          />
        </label>
        <label>Height:
          <input
            type="number"
            min={2}
            max={30}
            value={draftSettings.height}
            onChange={e => setDraftSettings(prev => ({...prev, height: Number(e.target.value)}))}
            style={{width: 70}}
          />
        </label>
        <label>Layers (3D depth):
          <input
            type="number"
            min={1}
            max={12}
            value={draftSettings.depth}
            onChange={e => setDraftSettings(prev => ({...prev, depth: Number(e.target.value)}))}
            title="Number of board layers used when 3D mode is enabled"
            style={{width: 70}}
          />
        </label>
        <label>Rack size:
          <input
            type="number"
            min={1}
            max={14}
            value={draftSettings.rackSize}
            onChange={e => setDraftSettings(prev => ({...prev, rackSize: Number(e.target.value)}))}
            style={{width: 70}}
          />
        </label>
        <label>Shape:
          <select
            value={draftSettings.shape}
            onChange={e => setDraftSettings(prev => ({...prev, shape: e.target.value as BoardShape}))}
          >
            <option value="rect">Full grid</option>
            <option value="diamond">Diamond</option>
            <option value="cross">Cross</option>
          </select>
        </label>
        <details style={{fontSize: 12}}>
          <summary style={{cursor: 'pointer', fontWeight: 600}}>Tile pool editor</summary>
          <div style={{marginTop: 8, display: 'flex', flexDirection: 'column', gap: 12}}>
            <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 8}}>
              <span style={{fontSize: 12, color: palette.textSubtle}}>Adjust tile counts and scores.</span>
              <button type="button" onClick={handleResetTiles} style={{fontSize: 12}}>Reset to defaults</button>
            </div>
            <div style={{overflowX: 'auto'}}>
              <table style={{width: '100%', borderCollapse: 'collapse', minWidth: 320, fontSize: 13}}>
                <thead>
                  <tr>
                    <th style={{textAlign: 'left', padding: '4px 6px', borderBottom: `1px solid ${palette.panelBorder}`}}>Tile</th>
                    <th style={{textAlign: 'left', padding: '4px 6px', borderBottom: `1px solid ${palette.panelBorder}`}}>Count</th>
                    <th style={{textAlign: 'left', padding: '4px 6px', borderBottom: `1px solid ${palette.panelBorder}`}}>Score</th>
                    <th style={{padding: '4px 6px', borderBottom: `1px solid ${palette.panelBorder}`}}></th>
                  </tr>
                </thead>
                <tbody>
                  {draftTileRows.map(row => (
                    <tr key={row.id}>
                      <td style={{padding: '4px 6px'}}>{row.id}</td>
                      <td style={{padding: '4px 6px', width: 90}}>
                        <input
                          type="number"
                          min={0}
                          value={row.count ?? 0}
                          onChange={e => updateTileEntry(row.id, {count: Number(e.target.value)})}
                          style={{width: '100%'}}
                        />
                      </td>
                      <td style={{padding: '4px 6px', width: 90}}>
                        <input
                          type="number"
                          value={row.score ?? 0}
                          onChange={e => updateTileEntry(row.id, {score: Number(e.target.value)})}
                          style={{width: '100%'}}
                        />
                      </td>
                      <td style={{padding: '4px 6px', textAlign: 'right'}}>
                        <button
                          type="button"
                          onClick={() => removeTile(row.id)}
                          disabled={draftTileRows.length <= 1}
                          style={{fontSize: 12}}
                        >
                          Remove
                        </button>
                      </td>
                    </tr>
                  ))}
                  <tr>
                    <td style={{padding: '4px 6px'}}>
                      <input
                        type="text"
                        value={newTileId}
                        onChange={e => setNewTileId(e.target.value.toUpperCase().replace(/[^A-Z0-9_]/g, ''))}
                        placeholder="ID"
                        style={{width: '100%'}}
                      />
                    </td>
                    <td style={{padding: '4px 6px'}}>
                      <input
                        type="number"
                        min={1}
                        value={newTileCount}
                        onChange={e => setNewTileCount(Number(e.target.value) || 1)}
                        style={{width: '100%'}}
                      />
                    </td>
                    <td style={{padding: '4px 6px'}}>
                      <input
                        type="number"
                        value={newTileScore}
                        onChange={e => setNewTileScore(Number(e.target.value) || 0)}
                        style={{width: '100%'}}
                      />
                    </td>
                    <td style={{padding: '4px 6px', textAlign: 'right'}}>
                      <button
                        type="button"
                        onClick={handleAddTile}
                        disabled={newTileId.trim() === ''}
                        style={{fontSize: 12}}
                      >
                        Add
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            {(tileCountsError || tileScoresError) && (
              <div style={{fontSize: 12, color: palette.errorBorder}}>
                {[tileCountsError, tileScoresError]
                  .filter(Boolean)
                  .map((msg, idx) => (
                    <div key={idx}>{msg}</div>
                  ))}
              </div>
            )}
          </div>
        </details>
        <div style={{fontSize: 12, color: palette.textSubtle}}>Layers are only used when 3D mode is enabled.</div>
        <button onClick={handleApplySettings}>Apply configuration</button>
      </ControlSection>

      <ControlSection title="Turn & Scores">
        <div style={{fontSize: 13}}>Turn {turnNumber + 1} • Player {activePlayer + 1}</div>
        <div style={{display: 'flex', gap: 8, flexWrap: 'wrap'}}>
          {players.length === 0 && <div style={{fontSize: 12, opacity: 0.7}}>Loading players…</div>}
          {players.map(player => (
            <div
              key={player.index}
              style={{
                padding: 8,
                minWidth: 120,
                borderRadius: 6,
                border: `1px solid ${palette.panelBorder}`,
                background: player.index === activePlayer ? palette.playerActiveBg : palette.playerBg,
              }}
            >
              <div style={{fontWeight: 600}}>Player {player.index + 1}</div>
              <div>{player.score} pts</div>
              <div style={{fontSize: 12, color: palette.textSubtle}}>Rack: {player.rack.join(' ') || '—'}</div>
            </div>
          ))}
        </div>
        <div style={{fontSize: 12, color: palette.textSubtle}}>
          Bag: {bagSummary.total} tiles{bagSummary.total > 0 && bagPreview ? ` • ${bagPreview}` : ''}
        </div>
      </ControlSection>

      <ControlSection title="Runtime & Board Layout">
        <label>
          <input
            type="checkbox"
            checked={useWorker}
            onChange={e => setUseWorker(e.target.checked)}
          />{' '}
          Use Web Worker
        </label>
        <label>
          <input
            type="checkbox"
            checked={useHex}
            onChange={e => {
              setUseHex(e.target.checked);
              if (e.target.checked) {
                setUseDiag(false);
                setUse3D(false);
                setDraftSettings(prev => ({...prev, shape: 'diamond'}));
              }
            }}
          />{' '}
          Hex adjacency
        </label>
        <label>
          <input
            type="checkbox"
            checked={useDiag}
            onChange={e => {
              setUseDiag(e.target.checked);
              if (e.target.checked) {
                setUseHex(false);
                setUse3D(false);
              }
            }}
          />{' '}
          Diagonal adjacency
        </label>
        <label>
          <input
            type="checkbox"
            checked={use3D}
            onChange={e => {
              setUse3D(e.target.checked);
              if (e.target.checked) {
                setUseHex(false);
              } else {
                setInfoMessage(null);
              }
            }}
          />{' '}
          3D (layers)
        </label>
        {useHex && (
          <div style={{fontSize: 12, color: palette.textSubtle}}>
            Hex adjacency uses staggered rows with three axes (E, NE, SE).
          </div>
        )}
        {useDiag && (
          <div style={{fontSize: 12, color: palette.textSubtle}}>
            Diagonal adjacency enables moves in all eight directions.
          </div>
        )}
        {use3D && (
          <>
            <span style={{fontSize: 12}}>Layer {z + 1} / {effectiveDepth}</span>
            <input
              type="range"
              min={0}
              max={Math.max(0, effectiveDepth - 1)}
              value={Math.min(z, Math.max(0, effectiveDepth - 1))}
              onChange={e => setZ(Number(e.target.value))}
            />
          </>
        )}
      </ControlSection>

      <ControlSection title="Dictionary & Language">
        <label>
          <input
            type="checkbox"
            checked={useDict}
            onChange={e => {
              const next = e.target.checked;
              setUseDict(next);
              setDictReady(!next);
              setDictError(null);
              setDictMessagesVersion(v => v + 1);
            }}
            disabled={dictLoading}
          />{' '}
          Dictionary checks
        </label>
        <label>
          Engine:{' '}
          <select
            value={dictEngine}
            onChange={e => {
              setDictEngine(e.target.value as 'fst' | 'set' | 'dawg' | 'gaddag');
              setDictMessagesVersion(v => v + 1);
            }}
            disabled={!useDict || dictLoading}
          >
            <option value="fst">FST</option>
            <option value="set">Set</option>
            <option value="dawg">DAWG</option>
            <option value="gaddag">GADDAG</option>
          </select>
        </label>
        <label title="Reorder placed tiles to any valid anagram on commit (row/column only)">
          <input
            type="checkbox"
            checked={useAnagram}
            onChange={e => setUseAnagram(e.target.checked)}
          />{' '}
          Anagram mode
        </label>
        <label>
          <input
            type="checkbox"
            checked={rtl}
            onChange={e => setRtl(e.target.checked)}
          />{' '}
          RTL reading
        </label>
        {dictLoading && <span data-testid="dictionary-loading" style={{fontSize: 12}}>Loading dictionary…</span>}
      </ControlSection>

      <ControlSection title="Stacking Rules">
        <label>
          <input
            type="checkbox"
            checked={stackOn}
            onChange={e => setStackOn(e.target.checked)}
          />{' '}
          Enable stacking
        </label>
        {stackOn && (
          <>
            <label>
              Scoring:{' '}
              <select value={stackScoring} onChange={e => setStackScoring(e.target.value as 'top' | 'sum')}>
                <option value="top">Top only</option>
                <option value="sum">Sum stack</option>
              </select>
            </label>
            <label>
              <input
                type="checkbox"
                checked={forbidSame}
                onChange={e => setForbidSame(e.target.checked)}
              />{' '}
              Forbid same overlay
            </label>
          </>
        )}
      </ControlSection>

      <ControlSection title="Move Controls">
        <button onClick={commitMove} disabled={pending.length === 0}>Commit move ({pending.length})</button>
        <button onClick={handlePendingReset} disabled={pending.length === 0}>Reset pending</button>
        <button onClick={undoMove} disabled={!game}>Undo</button>
        <button onClick={redoMove} disabled={!game}>Redo</button>
        <button
          onClick={() => {
            if (showMoves) {
              setShowMoves(false);
              setLegalMoves([]);
              setActiveMoveIndex(null);
            } else {
              fetchMoves();
            }
          }}
          disabled={!game || loadingMoves || (useDict && !dictReady) || use3D}
        >
          {showMoves ? 'Hide legal moves' : 'Show legal moves'}
        </button>
        {loadingMoves && <span style={{fontSize: 12}}>loading…</span>}
      </ControlSection>

      <ControlSection title="AI Assistant">
        <label>
          CPU difficulty:{' '}
          <select
            value={cpuDifficulty}
            onChange={e => setCpuDifficulty(e.target.value as 'off' | 'easy' | 'medium' | 'hard')}
            disabled={!dictReady || dictLoading || use3D}
          >
            <option value="off">Off</option>
            <option value="easy">Easy</option>
            <option value="medium">Medium</option>
            <option value="hard">Hard</option>
          </select>
        </label>
        <button
          onClick={requestCpuHint}
          disabled={!game || cpuDifficulty === 'off' || cpuThinking || !dictReady || use3D}
        >
          CPU hint
        </button>
        <button
          data-testid="cpu-play-button"
          onClick={playCpuSuggestion}
          disabled={!game || cpuSuggestion == null || cpuThinking || use3D}
        >
          Play as CPU
        </button>
        {cpuThinking && <span style={{fontSize: 12}}>computing…</span>}
        {cpuSuggestion && (
          <div
            style={{
              marginTop: 8,
              fontSize: 12,
              padding: 8,
              border: `1px solid ${palette.panelBorder}`,
              borderRadius: 4,
              background: colorMode === 'dark' ? 'rgba(30, 64, 175, 0.25)' : '#eff6ff',
            }}
          >
            <div style={{fontWeight: 600}}>CPU ({cpuSuggestion.difficulty}) suggests</div>
            <div><strong>{cpuSuggestion.word}</strong> — {cpuSuggestion.total} pts</div>
            <div style={{color: palette.textSubtle}}>Raw {cpuSuggestion.score}, leave {cpuSuggestion.rackLeave}, equity {cpuSuggestion.boardEquity}</div>
          </div>
        )}
        {!cpuSuggestion && lastCpu && (
          <div
            style={{
              marginTop: 8,
              fontSize: 12,
              padding: 8,
              border: `1px solid ${palette.panelBorder}`,
              borderRadius: 4,
              background: colorMode === 'dark' ? 'rgba(15, 118, 110, 0.2)' : '#ecfdf5',
            }}
          >
            <div style={{fontWeight: 600}}>Last CPU hint ({lastCpu.difficulty})</div>
            <div><strong>{lastCpu.word}</strong> — {lastCpu.total} pts</div>
            <div style={{color: palette.textSubtle}}>Raw {lastCpu.score}, leave {lastCpu.rackLeave}, equity {lastCpu.boardEquity}</div>
          </div>
        )}
      </ControlSection>

      <div style={{display: 'flex', gap: 16, alignItems: 'flex-start'}}>
        <div style={{display: 'grid', gridTemplateColumns: `repeat(${board.width}, 28px)`, gap: 4}}>
          {Array.from({length: layerHeight}).map((_, y) => (
            Array.from({length: board.width}).map((__, x) => {
              const globalY = use3D ? y + z * layerHeight : y;
              const key = `${x}-${globalY}`;
              const highlighted = highlightCells.has(`${x},${globalY}`);
              const activeCell = use3D || isCellActive(x, globalY);
              const cellStyle: React.CSSProperties = {
                width: 28,
                height: 28,
                border: `1px solid ${palette.boardCellBorder}`,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                position: 'relative',
                background: highlighted
                  ? palette.boardCellHighlight
                  : activeCell
                    ? palette.boardCellBg
                    : 'rgba(148, 163, 184, 0.15)',
                fontWeight: highlighted ? 600 : 400,
                opacity: activeCell ? 1 : 0.5,
                cursor: activeCell ? 'default' : 'not-allowed',
              };
              return (
                <div
                  key={key}
                  data-testid="playground-board-cell"
                  data-x={x}
                  data-y={globalY}
                  onDragOver={e => {
                    if (!activeCell) return;
                    e.preventDefault();
                  }}
                  onDrop={e => {
                    if (!activeCell) return;
                    onDropCell(x, y, e);
                  }}
                  style={cellStyle}
                  onMouseEnter={() => {
                    if (!showMoves) return;
                    const idx = legalMoves.findIndex(mv => mv.placements.some(p => p.x === x && p.y === globalY));
                    if (idx >= 0) setActiveMoveIndex(idx);
                  }}
                >
                  {(() => {
                    const tileId = cellDisplay(x, y);
                    if (!tileId) return null;
                    const meta = getTileMeta(tileId);
                    const symbol = meta?.symbol ?? tileId.slice(0, 1);
                    const score = meta?.score;
                    return (
                      <>
                        <span>{symbol.slice(0, 2)}</span>
                        {typeof score === 'number' && !Number.isNaN(score) && (
                          <span
                            style={{
                              position: 'absolute',
                              bottom: 2,
                              right: 3,
                              fontSize: 10,
                              fontWeight: 600,
                              opacity: 0.75,
                            }}
                          >
                            {score}
                          </span>
                        )}
                      </>
                    );
                  })()}
                </div>
              );
            })
          ))}
        </div>
        <div>
          <div style={{marginBottom: 6, fontSize: 12, opacity: 0.7}}>Active rack (Player {activePlayer + 1}) — drag onto board</div>
          <div style={{display: 'flex', gap: 6, flexWrap: 'wrap'}}>
            {availableRackTiles.map(({id, index}) => (
              <div
                key={`${id}-${index}`}
                draggable
                data-testid="playground-rack-tile"
                data-kind={id}
                onDragStart={e => onDragStartTile(id, e)}
                style={{
                  width: 32,
                  height: 32,
                  border: `1px solid ${palette.rackTileBorder}`,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  position: 'relative',
                  background: palette.rackTileBg,
                  cursor: 'grab',
                  fontWeight: 600,
                }}
              >
                {(() => {
                  const meta = getTileMeta(id);
                  const symbol = meta?.symbol ?? id.slice(0, 2);
                  const score = meta?.score;
                  return (
                    <>
                      <span>{symbol.slice(0, 2)}</span>
                      {typeof score === 'number' && !Number.isNaN(score) && (
                        <span
                          style={{
                            position: 'absolute',
                            bottom: 3,
                            right: 4,
                            fontSize: 11,
                            fontWeight: 600,
                            opacity: 0.75,
                          }}
                        >
                          {score}
                        </span>
                      )}
                    </>
                  );
                })()}
              </div>
            ))}
            {availableRackTiles.length === 0 && <div style={{fontSize: 12, color: palette.textSubtle}}>Rack empty or all tiles placed</div>}
          </div>
          {cpuSuggestion && (
            <div
              style={{
                marginTop: 12,
                fontSize: 12,
                padding: 8,
                border: `1px dashed ${palette.panelBorder}`,
                borderRadius: 4,
                background: colorMode === 'dark' ? 'rgba(56, 189, 248, 0.1)' : '#f8fbff',
              }}
            >
              <div style={{fontWeight: 600, marginBottom: 4}}>CPU placements preview</div>
              <div>{cpuSuggestion.placements.map(p => `(${p.x},${p.y})`).join(', ') || '—'}</div>
            </div>
          )}
        </div>
        {showMoves && (
          <div style={{minWidth: 200, maxWidth: 240, fontSize: 13}}>
            <div style={{fontWeight: 600, marginBottom: 8}}>Legal moves</div>
            {legalMoves.length === 0 && !loadingMoves && (
              <div style={{opacity: 0.7}}>No moves available for the current rack.</div>
            )}
            {legalMoves.map((mv, idx) => (
              <div
                key={`${mv.word}-${idx}`}
                data-testid={`legal-move-${idx}`}
                style={{marginBottom: 8, paddingBottom: 8, borderBottom: `1px solid ${palette.panelBorder}`}}
              >
                <div style={{fontWeight: 600}}>
                  #{idx + 1} {mv.word} <span style={{opacity: 0.7}}>({mv.total ?? mv.score} pts)</span>
                </div>
                <div style={{marginTop: 4, display: 'flex', gap: 6}}>
                  <button onClick={() => setActiveMoveIndex(idx)} style={{fontSize: 12}}>Highlight</button>
                  <button onClick={() => playGeneratedMove(mv)} style={{fontSize: 12}}>Play</button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div style={{display: 'flex', flexDirection: 'column', gap: 8}}>
        <div style={{fontSize: 12, fontWeight: 600}}>Snapshot JSON</div>
        <textarea
          value={snapshotText}
          onChange={e => setSnapshotText(e.target.value)}
          rows={4}
          style={{width: '100%', fontFamily: 'monospace'}}
          placeholder="Click Save snapshot to capture the current game state"
        />
        <div style={{display: 'flex', gap: 8}}>
          <button onClick={exportSnapshot} disabled={!game}>Save snapshot</button>
          <button onClick={importSnapshot} disabled={!game || snapshotText.trim() === ''}>Load snapshot</button>
          <button onClick={fetchEventLog} disabled={!game}>Show event log</button>
        </div>
        {eventLogText && (
          <pre
            style={{
              maxHeight: 200,
              overflow: 'auto',
              background: palette.panelBg,
              padding: 8,
              border: `1px solid ${palette.panelBorder}`,
            }}
          >
            {eventLogText}
          </pre>
        )}
      </div>
    </div>
  );
}
