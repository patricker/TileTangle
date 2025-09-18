import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import {classicTilesets} from './demoUtils';
import styles from './PlaygroundLayout.module.css';

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

type PanelProps = {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
  accent?: boolean;
  density?: 'spacious' | 'compact';
  children: React.ReactNode;
};

type StatChipProps = {
  label: string;
  value: React.ReactNode;
};

type SegmentedOption = {
  value: string;
  label: string;
  hint?: string;
  testId?: string;
};

type SegmentedControlProps = {
  name: string;
  value: string;
  options: SegmentedOption[];
  onChange: (value: string) => void;
};

type QuickPreset = {
  id: string;
  title: string;
  description: string;
  onApply: () => void;
};

function Panel({title, subtitle, actions, accent, density = 'spacious', children}: PanelProps): JSX.Element {
  const bodyClass = density === 'compact' ? styles.panelBodyCompact : styles.panelBody;
  return (
    <section className={`${styles.panel} ${accent ? styles.panelAccent : ''}`}>
      <div className={styles.panelHeader}>
        <div>
          <div className={styles.panelTitle}>{title}</div>
          {subtitle && <div className={styles.panelSubtitle}>{subtitle}</div>}
        </div>
        {actions && <div className={styles.panelActions}>{actions}</div>}
      </div>
      <div className={bodyClass}>{children}</div>
    </section>
  );
}

function StatChip({label, value}: StatChipProps): JSX.Element {
  return (
    <div className={styles.statChip}>
      <span className={styles.statChipLabel}>{label}</span>
      <span className={styles.statChipValue}>{value}</span>
    </div>
  );
}

function SegmentedControl({name, value, options, onChange}: SegmentedControlProps): JSX.Element {
  return (
    <div className={styles.segmentedControl} role="radiogroup" aria-label={name}>
      {options.map(option => {
        const active = option.value === value;
        return (
          <label
            key={option.value}
            className={styles.segmentedControlOption}
            data-active={active ? '1' : '0'}
          >
            <input
              type="radio"
              name={`segmented-${name}`}
              value={option.value}
              data-testid={option.testId}
              checked={active}
              onChange={() => onChange(option.value)}
            />
            <span>{option.label}</span>
            {option.hint && <small>{option.hint}</small>}
          </label>
        );
      })}
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
        shellBg: 'linear-gradient(122deg, rgba(15, 23, 42, 0.96) 0%, rgba(30, 64, 175, 0.45) 100%)',
        shellBorder: 'rgba(94, 234, 212, 0.22)',
        heroGradient: 'linear-gradient(135deg, rgba(59, 130, 246, 0.32) 0%, rgba(14, 165, 233, 0.28) 55%, rgba(56, 189, 248, 0.22) 100%)',
        heroText: '#e2e8f0',
        heroAccent: 'rgba(125, 211, 252, 0.55)',
        heroShadow: '0 40px 95px rgba(2, 6, 23, 0.6)',
        panelBg: 'rgba(15, 23, 42, 0.82)',
        panelBorder: 'rgba(148, 163, 184, 0.28)',
        panelShadow: '0 30px 60px rgba(2, 6, 23, 0.55)',
        playerActiveBg: 'rgba(56, 189, 248, 0.32)',
        playerBg: 'rgba(17, 24, 39, 0.55)',
        boardCellBg: 'rgba(15, 23, 42, 0.92)',
        boardCellBorder: 'rgba(148, 163, 184, 0.35)',
        boardCellHighlight: 'rgba(125, 211, 252, 0.4)',
        rackTileBg: 'rgba(30, 41, 59, 0.88)',
        rackTileBorder: 'rgba(148, 163, 184, 0.4)',
        rackTileHighlight: 'rgba(56, 189, 248, 0.38)',
        warningBg: 'rgba(234, 179, 8, 0.15)',
        warningBorder: 'rgba(250, 204, 21, 0.45)',
        errorBg: 'rgba(248, 113, 113, 0.16)',
        errorBorder: 'rgba(248, 113, 113, 0.6)',
        infoBg: 'rgba(56, 189, 248, 0.18)',
        infoBorder: 'rgba(129, 199, 212, 0.5)',
        accentBorder: 'rgba(56, 189, 248, 0.68)',
        textSubtle: 'rgba(226, 232, 240, 0.78)',
        statChipBg: 'rgba(30, 41, 59, 0.72)',
        statChipBorder: 'rgba(148, 196, 255, 0.42)',
        segmentedBg: 'rgba(17, 24, 39, 0.78)',
        segmentedBorder: 'rgba(71, 85, 105, 0.65)',
        segmentedActiveBg: 'rgba(56, 189, 248, 0.38)',
        segmentedActiveBorder: 'rgba(125, 211, 252, 0.65)',
        alertShadow: '0 18px 45px rgba(2, 6, 23, 0.55)',
      } as const;
    }
    return {
      shellBg: 'linear-gradient(128deg, rgba(248, 250, 252, 0.95) 0%, rgba(224, 242, 254, 0.9) 100%)',
      shellBorder: 'rgba(148, 163, 184, 0.28)',
      heroGradient: 'linear-gradient(135deg, rgba(59, 130, 246, 0.22) 0%, rgba(14, 165, 233, 0.18) 55%, rgba(2, 132, 199, 0.15) 100%)',
      heroText: '#0f172a',
      heroAccent: 'rgba(59, 130, 246, 0.45)',
      heroShadow: '0 30px 75px rgba(15, 23, 42, 0.25)',
      panelBg: 'rgba(255, 255, 255, 0.96)',
      panelBorder: 'rgba(148, 163, 184, 0.4)',
      panelShadow: '0 30px 60px rgba(15, 23, 42, 0.12)',
      playerActiveBg: 'rgba(147, 197, 253, 0.4)',
      playerBg: 'rgba(241, 245, 249, 0.9)',
      boardCellBg: '#ffffff',
      boardCellBorder: 'rgba(148, 163, 184, 0.38)',
      boardCellHighlight: 'rgba(59, 130, 246, 0.25)',
      rackTileBg: '#f8fafc',
      rackTileBorder: 'rgba(148, 163, 184, 0.45)',
      rackTileHighlight: 'rgba(59, 130, 246, 0.22)',
      warningBg: 'rgba(251, 191, 36, 0.2)',
      warningBorder: 'rgba(217, 119, 6, 0.4)',
      errorBg: 'rgba(248, 113, 113, 0.18)',
      errorBorder: 'rgba(220, 38, 38, 0.55)',
      infoBg: 'rgba(59, 130, 246, 0.14)',
      infoBorder: 'rgba(37, 99, 235, 0.4)',
      accentBorder: 'rgba(59, 130, 246, 0.6)',
      textSubtle: 'rgba(71, 85, 105, 0.9)',
      statChipBg: 'rgba(226, 232, 240, 0.72)',
      statChipBorder: 'rgba(148, 163, 184, 0.55)',
      segmentedBg: 'rgba(244, 247, 255, 0.9)',
      segmentedBorder: 'rgba(148, 163, 184, 0.5)',
      segmentedActiveBg: 'rgba(59, 130, 246, 0.2)',
      segmentedActiveBorder: 'rgba(37, 99, 235, 0.55)',
      alertShadow: '0 18px 40px rgba(15, 23, 42, 0.18)',
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
      const coordToIndex = new Map<string, number>();
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (!isActive(x, y)) continue;
          const idx = nodes.push({x, y}) - 1;
          coordToIndex.set(`${x},${y}`, idx);
        }
      }
      const edges: {a: number; b: number; dir: string}[] = [];
      const addEdge = (x1: number, y1: number, x2: number, y2: number, dir: string) => {
        if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
        if (!isActive(x1, y1) || !isActive(x2, y2)) return;
        const a = coordToIndex.get(`${x1},${y1}`);
        const b = coordToIndex.get(`${x2},${y2}`);
        if (a === undefined || b === undefined) return;
        edges.push({a, b, dir});
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
      const coordToIndex = new Map<string, number>();
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (!isActive(x, y)) continue;
          const idx = nodes.push({x, y}) - 1;
          coordToIndex.set(`${x},${y}`, idx);
        }
      }
      const edges: {a: number; b: number; dir: string}[] = [];
      const addEdge = (x1: number, y1: number, x2: number, y2: number, dir: string) => {
        if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
        if (!isActive(x1, y1) || !isActive(x2, y2)) return;
        const a = coordToIndex.get(`${x1},${y1}`);
        const b = coordToIndex.get(`${x2},${y2}`);
        if (a === undefined || b === undefined) return;
        edges.push({a, b, dir});
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
      const coordToIndex = new Map<string, number>();
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          if (!isActive(x, y)) continue;
          const idx = nodes.push({x, y}) - 1;
          coordToIndex.set(`${x},${y}`, idx);
        }
      }
      const edges: {a: number; b: number; dir: string}[] = [];
      const addEdge = (x1: number, y1: number, x2: number, y2: number, dir: string) => {
        if (x2 < 0 || x2 >= width || y2 < 0 || y2 >= height) return;
        if (!isActive(x1, y1) || !isActive(x2, y2)) return;
        const a = coordToIndex.get(`${x1},${y1}`);
        const b = coordToIndex.get(`${x2},${y2}`);
        if (a === undefined || b === undefined) return;
        edges.push({a, b, dir});
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
        if (!useWorker) {
          try {
            const mod = wasmModuleRef.current;
            if (mod && typeof mod.last_error === 'function') {
              const last = mod.last_error();
              if (last) {
                console.error('WASM last_error:', last);
              }
            }
          } catch (inner) {
            console.error('Failed to read WASM last_error', inner);
          }
        }
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

  useEffect(() => {
    return () => {
      terminateWorker();
    };
  }, [terminateWorker]);

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

  const applySettings = useCallback((settings: SetupState) => {
    const width = clamp(settings.width, 2, 30);
    const height = clamp(settings.height, 2, 30);
    const depth = clamp(settings.depth, 1, 12);
    const rackSize = clamp(settings.rackSize, 1, 14);
    const countsParsed = parseTileCounts(settings.tileCountsText, defaultTileCounts);
    if (countsParsed.error) {
      setTileCountsError(countsParsed.error);
      return;
    }
    const scoresParsed = parseTileScores(settings.tileScoresText, defaultTileScores);
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
      shape: settings.shape,
    });
    setDictMessagesVersion(v => v + 1);
  }, [defaultTileCounts, defaultTileScores]);

  const handleApplySettings = useCallback(() => {
    applySettings(draftSettings);
  }, [applySettings, draftSettings]);

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

  const adjacencyMode = useMemo(() => {
    if (useHex) return 'hex';
    if (useDiag) return 'diagonal';
    return 'orthogonal';
  }, [useHex, useDiag]);

  const handleAdjacencyModeChange = useCallback((mode: string) => {
    if (mode === 'hex') {
      setUseHex(true);
      setUseDiag(false);
      setUse3D(false);
      setDraftSettings(prev => ({...prev, shape: 'diamond'}));
      return;
    }
    if (mode === 'diagonal') {
      setUseDiag(true);
      setUseHex(false);
      return;
    }
    setUseHex(false);
    setUseDiag(false);
  }, [setUseDiag, setUseHex, setUse3D]);

  const dimensionMode = use3D ? '3d' : '2d';

  const handleDimensionModeChange = useCallback((mode: string) => {
    if (mode === '3d') {
      setUse3D(true);
      setUseHex(false);
      setUseDiag(false);
      return;
    }
    setUse3D(false);
    setZ(0);
    setInfoMessage(null);
  }, [setUse3D, setUseHex, setUseDiag, setZ, setInfoMessage]);

  const boardSizeLabel = useMemo(() => {
    const size = `${appliedSettings.width}×${appliedSettings.height}`;
    if (use3D) {
      return `${size}×${Math.max(1, appliedSettings.depth)}`;
    }
    return size;
  }, [appliedSettings.width, appliedSettings.height, appliedSettings.depth, use3D]);

  const adjacencyLabel = useMemo(() => {
    if (adjacencyMode === 'hex') return 'Hex graph';
    if (adjacencyMode === 'diagonal') return 'Diagonal';
    return 'Orthogonal';
  }, [adjacencyMode]);

  const dimensionLabel = useMemo(() => {
    if (use3D) {
      return `${Math.max(1, appliedSettings.depth)} layers`;
    }
    return '2D board';
  }, [use3D, appliedSettings.depth]);

  const cpuLabel = useMemo(() => {
    if (cpuDifficulty === 'off') return 'CPU disabled';
    return `CPU ${cpuDifficulty}`;
  }, [cpuDifficulty]);

  const heroStats = useMemo(() => [
    {label: 'Board', value: boardSizeLabel},
    {label: 'Adjacency', value: adjacencyLabel},
    {label: 'Dimensions', value: dimensionLabel},
    {label: 'Rack', value: `${appliedSettings.rackSize} tiles`},
  ], [boardSizeLabel, adjacencyLabel, dimensionLabel, appliedSettings.rackSize]);

  const quickPresets = useMemo<QuickPreset[]>(() => [
    {
      id: 'classic',
      title: 'Classic 15×15',
      description: 'Crossword feel with orthogonal adjacency and rack of seven.',
      onApply: () => {
        const next: SetupState = {
          width: 15,
          height: 15,
          depth: 1,
          rackSize: 7,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'rect',
        };
        setDraftSettings(next);
        setUseHex(false);
        setUseDiag(false);
        setUse3D(false);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        applySettings(next);
      },
    },
    {
      id: 'hex',
      title: 'Hex Garden',
      description: 'Diamond mask with full hex connectivity.',
      onApply: () => {
        const next: SetupState = {
          width: Math.max(7, appliedSettings.width),
          height: Math.max(7, appliedSettings.height),
          depth: 1,
          rackSize: appliedSettings.rackSize,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'diamond',
        };
        setDraftSettings(next);
        setUseHex(true);
        setUseDiag(false);
        setUse3D(false);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        applySettings(next);
      },
    },
    {
      id: 'speed',
      title: 'Sprint 11×11',
      description: 'Smaller board, six-tile rack, great for quick rounds.',
      onApply: () => {
        const next: SetupState = {
          width: 11,
          height: 11,
          depth: 1,
          rackSize: 6,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'rect',
        };
        setDraftSettings(next);
        setUseHex(false);
        setUseDiag(true);
        setUse3D(false);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        applySettings(next);
      },
    },
    {
      id: 'stacked',
      title: '3D Tower',
      description: 'Nine-by-nine with layered play; perfect for stack mode.',
      onApply: () => {
        const next: SetupState = {
          width: 9,
          height: 9,
          depth: Math.max(3, appliedSettings.depth),
          rackSize: appliedSettings.rackSize,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'rect',
        };
        setDraftSettings(next);
        setUseHex(false);
        setUseDiag(false);
        setUse3D(true);
        setZ(0);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        applySettings(next);
      },
    },
  ], [
    appliedSettings.depth,
    appliedSettings.height,
    appliedSettings.rackSize,
    appliedSettings.width,
    applySettings,
    draftSettings.tileCountsText,
    draftSettings.tileScoresText,
    setUseHex,
    setUseDiag,
    setUse3D,
    setCpuSuggestion,
    setLastCpu,
    setPending,
    setDraftSettings,
    setZ,
  ]);

  const themeVars = useMemo(() => ({
    '--tt-shell-bg': palette.shellBg,
    '--tt-shell-border': palette.shellBorder,
    '--tt-hero-gradient': palette.heroGradient,
    '--tt-hero-text': palette.heroText,
    '--tt-hero-accent': palette.heroAccent,
    '--tt-hero-shadow': palette.heroShadow,
    '--tt-panel-bg': palette.panelBg,
    '--tt-panel-border': palette.panelBorder,
    '--tt-panel-shadow': palette.panelShadow,
    '--tt-accent-border': palette.accentBorder,
    '--tt-text-subtle': palette.textSubtle,
    '--tt-stat-chip-bg': palette.statChipBg,
    '--tt-stat-chip-border': palette.statChipBorder,
    '--tt-segmented-bg': palette.segmentedBg,
    '--tt-segmented-border': palette.segmentedBorder,
    '--tt-segmented-active-bg': palette.segmentedActiveBg,
    '--tt-segmented-active-border': palette.segmentedActiveBorder,
    '--tt-alert-error-bg': palette.errorBg,
    '--tt-alert-error-border': palette.errorBorder,
    '--tt-alert-warning-bg': palette.warningBg,
    '--tt-alert-warning-border': palette.warningBorder,
    '--tt-alert-info-bg': palette.infoBg,
    '--tt-alert-info-border': palette.infoBorder,
    '--tt-alert-shadow': palette.alertShadow,
  }) as React.CSSProperties, [palette]);

  const layerHeight = use3D && board ? Math.floor(board.height / Math.max(1, effectiveDepth)) : board?.height ?? 0;

  if (!ready || !board) {
    return <div className={styles.loading}>Loading WASM…</div>;
  }

  return (
    <div className={styles.shell} style={themeVars}>
      <header className={styles.hero}>
        <div className={styles.heroCopy}>
          <div className={styles.heroEyebrow}>TileTangle Playground</div>
          <h2 className={styles.heroTitle}>Design. Experiment. Solve.</h2>
          <p className={styles.heroDescription}>
            Tune adjacency, stack rules, and automation to watch the engine reshape every move in real time.
          </p>
          <div className={styles.heroStatsRow}>
            {heroStats.map(stat => (
              <StatChip key={stat.label} label={stat.label} value={stat.value} />
            ))}
            <StatChip label="Automation" value={cpuLabel} />
          </div>
        </div>
        <div className={styles.heroPresets}>
          <div className={styles.presetsHeading}>Quick presets</div>
          <div className={styles.presetGrid}>
            {quickPresets.map(preset => (
              <button
                key={preset.id}
                type="button"
                className={styles.presetCard}
                onClick={preset.onApply}
              >
                <span className={styles.presetTitle}>{preset.title}</span>
                <span className={styles.presetDescription}>{preset.description}</span>
              </button>
            ))}
          </div>
        </div>
      </header>
      {(errorMessage || dictError || cpuError || infoMessage) && (
        <div className={styles.alertStack}>
          {errorMessage && <div className={`${styles.alert} ${styles.alertError}`}>{errorMessage}</div>}
          {dictError && <div className={`${styles.alert} ${styles.alertWarning}`}>{dictError}</div>}
          {cpuError && <div className={`${styles.alert} ${styles.alertWarning}`}>{cpuError}</div>}
          {infoMessage && <div className={`${styles.alert} ${styles.alertInfo}`}>{infoMessage}</div>}
        </div>
      )}

      <div className={styles.layout}>
        <aside className={styles.sidebar}>
          <Panel
            title="Board Setup"
            subtitle="Adjust dimensions and masks, then relaunch with Apply."
            actions={
              <button type="button" className={styles.applyButton} onClick={handleApplySettings}>
                Apply configuration
              </button>
            }
            density="compact"
          >
            <div className={styles.fieldGrid}>
              <label className={styles.field}>
                <span>Width</span>
                <input
                  type="number"
                  min={2}
                  max={30}
                  value={draftSettings.width}
                  onChange={e => setDraftSettings(prev => ({...prev, width: Number(e.target.value)}))}
                />
              </label>
              <label className={styles.field}>
                <span>Height</span>
                <input
                  type="number"
                  min={2}
                  max={30}
                  value={draftSettings.height}
                  onChange={e => setDraftSettings(prev => ({...prev, height: Number(e.target.value)}))}
                />
              </label>
              <label className={styles.field} title="Number of layers when 3D mode is active">
                <span>Layers</span>
                <input
                  type="number"
                  min={1}
                  max={12}
                  value={draftSettings.depth}
                  onChange={e => setDraftSettings(prev => ({...prev, depth: Number(e.target.value)}))}
                />
              </label>
              <label className={styles.field}>
                <span>Rack size</span>
                <input
                  type="number"
                  min={1}
                  max={14}
                  value={draftSettings.rackSize}
                  onChange={e => setDraftSettings(prev => ({...prev, rackSize: Number(e.target.value)}))}
                />
              </label>
            </div>
            <label className={styles.field}>
              <span>Shape</span>
              <select
                value={draftSettings.shape}
                data-testid="board-shape-select"
                onChange={e => setDraftSettings(prev => ({...prev, shape: e.target.value as BoardShape}))}
              >
                <option value="rect">Full grid</option>
                <option value="diamond">Diamond</option>
                <option value="cross">Cross</option>
              </select>
            </label>
            <div className={styles.helperText}>Layers only apply when 3D mode is enabled.</div>
          </Panel>

          <Panel title="Tile Pool" subtitle="Fine-tune counts and scoring." density="compact">
            <div className={styles.tileEditor}>
              <div className={styles.tileEditorHeader}>
                <span>Adjust distribution.</span>
                <button type="button" className={styles.linkButton} onClick={handleResetTiles}>
                  Reset defaults
                </button>
              </div>
              <div className={styles.tileTableWrapper}>
                <table className={styles.tileTable}>
                  <thead>
                    <tr>
                      <th>Tile</th>
                      <th>Count</th>
                      <th>Score</th>
                      <th aria-hidden="true"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {draftTileRows.map(row => (
                      <tr key={row.id}>
                        <td>{row.id}</td>
                        <td>
                          <input
                            type="number"
                            min={0}
                            value={row.count ?? 0}
                            onChange={e => updateTileEntry(row.id, {count: Number(e.target.value)})}
                          />
                        </td>
                        <td>
                          <input
                            type="number"
                            value={row.score ?? 0}
                            onChange={e => updateTileEntry(row.id, {score: Number(e.target.value)})}
                          />
                        </td>
                        <td className={styles.tableActions}>
                          <button type="button" onClick={() => removeTile(row.id)} disabled={draftTileRows.length <= 1}>
                            Remove
                          </button>
                        </td>
                      </tr>
                    ))}
                    <tr>
                      <td>
                        <input
                          type="text"
                          value={newTileId}
                          onChange={e => setNewTileId(e.target.value.toUpperCase().replace(/[^A-Z0-9_]/g, ''))}
                          placeholder="ID"
                        />
                      </td>
                      <td>
                        <input
                          type="number"
                          min={1}
                          value={newTileCount}
                          onChange={e => setNewTileCount(Number(e.target.value) || 1)}
                        />
                      </td>
                      <td>
                        <input
                          type="number"
                          value={newTileScore}
                          onChange={e => setNewTileScore(Number(e.target.value) || 0)}
                        />
                      </td>
                      <td className={styles.tableActions}>
                        <button type="button" onClick={handleAddTile} disabled={newTileId.trim() === ''}>
                          Add
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              {(tileCountsError || tileScoresError) && (
                <div className={styles.errorText}>
                  {[tileCountsError, tileScoresError]
                    .filter(Boolean)
                    .map((msg, idx) => (
                      <div key={idx}>{msg}</div>
                    ))}
                </div>
              )}
            </div>
          </Panel>

          <Panel title="Language & Dictionary" subtitle="Guard rails for move validation." density="compact">
            <label className={styles.toggleRow}>
              <input type="checkbox" checked={useDict} onChange={e => setUseDict(e.target.checked)} />
              <span>Use dictionary validation</span>
            </label>
            <label className={styles.field}>
              <span>Engine</span>
              <select
                value={dictEngine}
                onChange={e => setDictEngine(e.target.value as 'fst' | 'set' | 'dawg' | 'gaddag')}
                disabled={!useDict || dictLoading}
              >
                <option value="fst">FST</option>
                <option value="set">Set</option>
                <option value="dawg">DAWG</option>
                <option value="gaddag">GADDAG</option>
              </select>
            </label>
            <label className={styles.toggleRow} title="Reorder placed tiles into any valid anagram when committing the move">
              <input type="checkbox" checked={useAnagram} onChange={e => setUseAnagram(e.target.checked)} />
              <span>Anagram commit</span>
            </label>
            <label className={styles.toggleRow}>
              <input type="checkbox" checked={rtl} onChange={e => setRtl(e.target.checked)} />
              <span>RTL reading direction</span>
            </label>
            {dictLoading && (
              <span data-testid="dictionary-loading" className={styles.helperText}>
                Loading dictionary…
              </span>
            )}
          </Panel>

          <Panel title="Stacking Rules" subtitle="Experiment with layered tiles." density="compact">
            <label className={styles.toggleRow}>
              <input type="checkbox" checked={stackOn} onChange={e => setStackOn(e.target.checked)} />
              <span>Enable stacking</span>
            </label>
            {stackOn && (
              <div className={styles.fieldStack}>
                <label className={styles.field}>
                  <span>Scoring</span>
                  <select value={stackScoring} onChange={e => setStackScoring(e.target.value as 'top' | 'sum')}>
                    <option value="top">Top only</option>
                    <option value="sum">Sum stack</option>
                  </select>
                </label>
                <label className={styles.toggleRow}>
                  <input type="checkbox" checked={forbidSame} onChange={e => setForbidSame(e.target.checked)} />
                  <span>Forbid identical overlays</span>
                </label>
              </div>
            )}
          </Panel>
        </aside>

        <main className={styles.stage}>
          <Panel
            title="Board"
            subtitle={`Turn ${turnNumber + 1} • Player ${activePlayer + 1}`}
            accent
          >
            <div className={styles.boardWrapper}>
              <div
                className={styles.boardGrid}
                style={{gridTemplateColumns: `repeat(${board.width}, 28px)`}}
              >
                {Array.from({length: layerHeight}).map((_, y) => (
                  Array.from({length: board.width}).map((__, x) => {
                    const globalY = use3D ? y + z * layerHeight : y;
                    const key = `${x}-${globalY}`;
                    const highlighted = highlightCells.has(`${x},${globalY}`);
                    const activeCell = use3D || isCellActive(x, globalY);
                    const tileId = cellDisplay(x, y);
                    const meta = tileId ? getTileMeta(tileId) : null;
                    const symbol = meta?.symbol ?? tileId?.slice(0, 2) ?? '';
                    const score = meta?.score;
                    const cellStyle: React.CSSProperties = {
                      border: `1px solid ${palette.boardCellBorder}`,
                      background: highlighted
                        ? palette.boardCellHighlight
                        : activeCell
                          ? palette.boardCellBg
                          : 'rgba(148, 163, 184, 0.12)',
                      fontWeight: highlighted ? 600 : 500,
                      opacity: activeCell ? 1 : 0.55,
                      cursor: activeCell ? 'default' : 'not-allowed',
                    };
                    return (
                      <div
                        key={key}
                        data-testid="playground-board-cell"
                        data-x={x}
                        data-y={globalY}
                        data-active={activeCell ? '1' : '0'}
                        className={styles.boardCell}
                        style={cellStyle}
                        onDragOver={e => {
                          if (!activeCell) return;
                          e.preventDefault();
                        }}
                        onDrop={e => {
                          if (!activeCell) return;
                          onDropCell(x, y, e);
                        }}
                        onMouseEnter={() => {
                          if (!showMoves) return;
                          const idx = legalMoves.findIndex(mv => mv.placements.some(p => p.x === x && p.y === globalY));
                          if (idx >= 0) setActiveMoveIndex(idx);
                        }}
                      >
                        {symbol && <span>{symbol.slice(0, 2)}</span>}
                        {typeof score === 'number' && !Number.isNaN(score) && (
                          <span className={styles.boardCellScore}>{score}</span>
                        )}
                      </div>
                    );
                  })
                ))}
              </div>
            </div>
            {use3D && (
              <label className={styles.layerSlider}>
                <span>Viewing layer {z + 1} / {effectiveDepth}</span>
                <input
                  type="range"
                  min={0}
                  max={Math.max(0, effectiveDepth - 1)}
                  value={z}
                  onChange={e => setZ(Number(e.target.value))}
                />
              </label>
            )}
          </Panel>

          <div className={styles.stageSplit}>
            <Panel title={`Active Rack • Player ${activePlayer + 1}`} density="compact">
              <div className={styles.rackRow}>
                {availableRackTiles.map(({id, index}) => {
                  const meta = getTileMeta(id);
                  const symbol = meta?.symbol ?? id.slice(0, 2);
                  const score = meta?.score;
                  return (
                    <div
                      key={`${id}-${index}`}
                      draggable
                      data-testid="playground-rack-tile"
                      data-kind={id}
                      className={styles.rackTile}
                      onDragStart={e => onDragStartTile(id, e)}
                    >
                      <span>{symbol.slice(0, 2)}</span>
                      {typeof score === 'number' && !Number.isNaN(score) && (
                        <span className={styles.rackTileScore}>{score}</span>
                      )}
                    </div>
                  );
                })}
                {availableRackTiles.length === 0 && (
                  <div className={styles.helperText}>Rack empty or all tiles placed.</div>
                )}
              </div>
              {cpuSuggestion && (
                <div className={styles.cpuPreview}>
                  <div className={styles.cpuPreviewTitle}>CPU placements preview</div>
                  <div>{cpuSuggestion.placements.map(p => `(${p.x},${p.y})`).join(', ') || '—'}</div>
                </div>
              )}
            </Panel>

            <Panel title="Move Controls" density="compact">
              <div className={styles.buttonRow}>
                <button onClick={commitMove} disabled={pending.length === 0}>
                  Commit move ({pending.length})
                </button>
                <button onClick={handlePendingReset} disabled={pending.length === 0}>
                  Reset pending
                </button>
              </div>
              <div className={styles.buttonRow}>
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
              </div>
              {loadingMoves && <div className={styles.helperText}>Loading legal moves…</div>}
            </Panel>
          </div>
        </main>

        <aside className={styles.rightRail}>
          <Panel title="Turn Tracker" subtitle="Scoreboard and bag" density="compact">
            <div className={styles.playerList}>
              {players.length === 0 && <div className={styles.helperText}>Loading players…</div>}
              {players.map(player => (
                <div
                  key={player.index}
                  className={`${styles.playerCard} ${player.index === activePlayer ? styles.playerCardActive : ''}`}
                >
                  <div className={styles.playerCardHeader}>Player {player.index + 1}</div>
                  <div className={styles.playerScore}>{player.score} pts</div>
                  <div className={styles.playerRack}>{player.rack.join(' ') || '—'}</div>
                </div>
              ))}
            </div>
            <div className={styles.helperText}>
              Bag: {bagSummary.total} tiles{bagSummary.total > 0 && bagPreview ? ` • ${bagPreview}` : ''}
            </div>
          </Panel>

          <Panel title="Playground Modes" subtitle="Switch adjacency and runtime modes." density="compact">
            <div className={styles.modeGroup}>
              <div className={styles.modeLabel}>Adjacency</div>
              <SegmentedControl
                name="Adjacency"
                value={adjacencyMode}
                onChange={handleAdjacencyModeChange}
                options={[
                  {value: 'orthogonal', label: 'Orthogonal', hint: 'Classic'},
                  {value: 'diagonal', label: 'Diagonal', hint: 'Eight-way'},
                  {value: 'hex', label: 'Hex', hint: 'Three axes', testId: 'toggle-hex-adjacency'},
                ]}
              />
            </div>
            <div className={styles.modeGroup}>
              <div className={styles.modeLabel}>Dimensions</div>
              <SegmentedControl
                name="Dimensions"
                value={dimensionMode}
                onChange={handleDimensionModeChange}
                options={[
                  {value: '2d', label: '2D'},
                  {value: '3d', label: '3D', hint: 'Stacked'},
                ]}
              />
            </div>
            <label className={styles.toggleRow}>
              <input type="checkbox" checked={useWorker} onChange={e => setUseWorker(e.target.checked)} />
              <span>Run heavy work in a Web Worker</span>
            </label>
            {adjacencyMode === 'hex' && (
              <div className={styles.helperText}>Hex adjacency uses staggered rows with three axes (E, NE, SE).</div>
            )}
            {adjacencyMode === 'diagonal' && (
              <div className={styles.helperText}>Diagonal mode enables moves along all eight directions.</div>
            )}
          </Panel>

          <Panel title="Automation & Hints" subtitle="Let the engine explore." density="compact">
            <SegmentedControl
              name="CPU Difficulty"
              value={cpuDifficulty}
              onChange={value => setCpuDifficulty(value as 'off' | 'easy' | 'medium' | 'hard')}
              options={[
                {value: 'off', label: 'Off'},
                {value: 'easy', label: 'Easy'},
                {value: 'medium', label: 'Medium'},
                {value: 'hard', label: 'Hard'},
              ]}
            />
            <div className={styles.buttonRow}>
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
            </div>
            {cpuThinking && <div className={styles.helperText}>Computing best move…</div>}
            {cpuSuggestion && (
              <div className={styles.cpuCard}>
                <div className={styles.cpuCardTitle}>CPU ({cpuSuggestion.difficulty}) suggests</div>
                <div className={styles.cpuCardBody}>
                  <strong>{cpuSuggestion.word}</strong> — {cpuSuggestion.total} pts
                </div>
                <div className={styles.cpuCardMeta}>Raw {cpuSuggestion.score}, leave {cpuSuggestion.rackLeave}, equity {cpuSuggestion.boardEquity}</div>
              </div>
            )}
            {!cpuSuggestion && lastCpu && (
              <div className={styles.cpuCardMuted}>
                <div className={styles.cpuCardTitle}>Last hint ({lastCpu.difficulty})</div>
                <div className={styles.cpuCardBody}>
                  <strong>{lastCpu.word}</strong> — {lastCpu.total} pts
                </div>
                <div className={styles.cpuCardMeta}>Raw {lastCpu.score}, leave {lastCpu.rackLeave}, equity {lastCpu.boardEquity}</div>
              </div>
            )}
          </Panel>

          <Panel
            title="Explore Moves"
            subtitle="Generate legal plays for the current rack."
            density="compact"
            accent={showMoves}
          >
            <div className={styles.buttonRow}>
              <button
                onClick={() => fetchMoves()}
                disabled={!game || loadingMoves || (useDict && !dictReady) || use3D}
              >
                {showMoves ? 'Refresh legal moves' : 'Show legal moves'}
              </button>
              {showMoves && (
                <button
                  onClick={() => {
                    setShowMoves(false);
                    setLegalMoves([]);
                    setActiveMoveIndex(null);
                  }}
                >
                  Hide list
                </button>
              )}
            </div>
            {showMoves && (
              <div className={styles.movesList}>
                {legalMoves.length === 0 && !loadingMoves && (
                  <div className={styles.helperText}>No moves available for the current rack.</div>
                )}
                {legalMoves.map((mv, idx) => (
                  <div key={`${mv.word}-${idx}`} data-testid={`legal-move-${idx}`} className={styles.moveCard}>
                    <div className={styles.moveHeader}>
                      <span>#{idx + 1}</span>
                      <strong>{mv.word}</strong>
                      <span className={styles.moveScore}>{mv.total ?? mv.score} pts</span>
                    </div>
                    <div className={styles.buttonRow}>
                      <button type="button" onClick={() => setActiveMoveIndex(idx)}>
                        Highlight
                      </button>
                      <button type="button" onClick={() => playGeneratedMove(mv)}>
                        Play move
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
            {loadingMoves && <div className={styles.helperText}>Loading legal moves…</div>}
          </Panel>

          <Panel title="Snapshots & Log" subtitle="Export state or inspect history." density="compact">
            <textarea
              value={snapshotText}
              onChange={e => setSnapshotText(e.target.value)}
              rows={5}
              className={styles.snapshotArea}
              placeholder="Click Save snapshot to capture the current game state"
            />
            <div className={styles.buttonRow}>
              <button onClick={exportSnapshot} disabled={!game}>Save snapshot</button>
              <button onClick={importSnapshot} disabled={!game || snapshotText.trim() === ''}>Load snapshot</button>
              <button onClick={fetchEventLog} disabled={!game}>Show event log</button>
            </div>
            {eventLogText && (
              <pre className={styles.logViewer}>{eventLogText}</pre>
            )}
          </Panel>
        </aside>
      </div>
    </div>
  );
}
