import React, {useCallback, useEffect, useMemo, useRef, useState} from 'react';
import {useColorMode} from '@docusaurus/theme-common';
import {classicTilesets} from './demoUtils';
import PlaygroundBoard, {HEX_POLYGON} from './playground/PlaygroundBoard';
import PlaygroundHero from './playground/PlaygroundHero';
import PlaygroundShell from './playground/PlaygroundShell';
import AlertStack, {type AlertItem} from './playground/AlertStack';
import type {BoardJson} from './playground/types';
import {SetupState, buildShapeMask, clamp, formatTileCounts, formatTileScores, parseTileCounts, parseTileScores} from './playground/config';
import {resolveBonusPreset, type BonusCell} from './playground/bonuses';
import {BoardSetupPanel, LanguagePanel, StackingPanel} from './playground/panels';
import {ButtonRow, type ButtonConfig, CpuHintSummary, MoveList, Panel, PlayerList, RackRow, ToggleField, type RackRowTile, SegmentedControl, type SegmentedOption} from './playground/ui';
import {buildThemeVars, getPlaygroundPalette} from './playground/theme';
import {useWorkerMessenger} from './playground/useWorkerMessenger';
import {randomSeed} from './playground/random';
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

const dictionaryTextSources = ['/dictionaries/TWL06.txt', '/dictionaries/demo.txt'];

type DictionaryPayload =
  | {kind: 'fst'; bytes: Uint8Array}
  | {kind: 'text'; text: string};


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

type QuickPreset = {
  id: string;
  title: string;
  description: string;
  onApply: () => void;
};

export default function Playground({initial}: PlaygroundProps = {}): JSX.Element {
  const initialConfig = initial?.config;
  const initialLayout = (initialConfig?.board_layout ?? {}) as Record<string, any>;
  const initialWidth = typeof initialLayout.width === 'number' && initialLayout.width > 0 ? initialLayout.width : 9;
  const initialHeight = typeof initialLayout.height === 'number' && initialLayout.height > 0 ? initialLayout.height : 9;
  const initialDepth = typeof initialLayout.depth === 'number' && initialLayout.depth > 0 ? initialLayout.depth : (initial?.depth ?? 1);
  const initialRackSize = initialConfig?.rack_size ?? 7;

  const {colorMode} = useColorMode();

  const palette = useMemo(() => getPlaygroundPalette(colorMode as 'light' | 'dark'), [colorMode]);

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
  const [draftAdjacencyMode, setDraftAdjacencyMode] = useState<'orthogonal' | 'diagonal' | 'hex'>(
    initial?.useHex ? 'hex' : initial?.useDiag ? 'diagonal' : 'orthogonal',
  );
  const [draftDimensionMode, setDraftDimensionMode] = useState<'2d' | '3d'>(initial?.use3D ? '3d' : '2d');
  const [rtl, setRtl] = useState(initial?.rtl ?? false);
  const [stackOn, setStackOn] = useState(initial?.stackOn ?? false);
  const [stackScoring, setStackScoring] = useState<'top' | 'sum'>(initial?.stackScoring ?? 'top');
  const [forbidSame, setForbidSame] = useState(initial?.forbidSame ?? true);
  const [z, setZ] = useState(0);
  const rackOverrideRef = useRef<string[] | undefined>(initial?.rack);
  const wasmModuleRef = useRef<any | null>(null);
  const wasmModulePromiseRef = useRef<Promise<any> | null>(null);
  const {ensureWorker, callWorker, terminateWorker} = useWorkerMessenger();

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

  const [appliedSettings, setAppliedSettings] = useState<SetupState>({
    width: initialWidth,
    height: initialHeight,
    depth: initialDepth,
    rackSize: initialRackSize,
    tileCountsText: initialTileCountsText,
    tileScoresText: initialTileScoresText,
    shape: 'rect',
    bonusPreset: 'auto',
  });
  const [draftSettings, setDraftSettings] = useState<SetupState>(appliedSettings);
  const handleDraftSettingsChange = useCallback((updates: Partial<SetupState>) => {
    setDraftSettings(prev => ({...prev, ...updates}));
  }, []);
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

  const fetchDictionaryPayload = useCallback(
    async (engine: 'fst' | 'set' | 'dawg' | 'gaddag'): Promise<DictionaryPayload> => {
      if (engine === 'fst') {
        try {
          const resp = await fetch('/dictionaries/TWL06.fst');
          if (resp.ok) {
            const bytes = new Uint8Array(await resp.arrayBuffer());
            return {kind: 'fst', bytes};
          }
          console.warn('TWL06.fst fetch returned status', resp.status);
        } catch (err) {
          console.warn('FST dictionary fetch failed', err);
        }
      }

      for (const url of dictionaryTextSources) {
        try {
          const resp = await fetch(url);
          if (!resp.ok) {
            console.warn(`Dictionary fetch from ${url} returned status`, resp.status);
            continue;
          }
          const text = await resp.text();
          return {kind: 'text', text};
        } catch (err) {
          console.warn(`Dictionary fetch failed from ${url}`, err);
        }
      }

      return {kind: 'text', text: fallbackDictionaryText};
    },
    [],
  );

  const applyDictionaryToWorker = useCallback(
    async (
      call: (action: string, payload?: any) => Promise<any>,
      engine: 'fst' | 'set' | 'dawg' | 'gaddag',
      shouldUseDict: boolean,
    ): Promise<boolean> => {
      try {
        if (!shouldUseDict) {
          await call('set_free_word_mode', {on: true});
          return true;
        }
        const payload = await fetchDictionaryPayload(engine);
        if (engine === 'fst' && payload.kind === 'fst') {
          await call('set_dictionary_from_fst_bytes', {bytes: payload.bytes, case_fold: true});
        } else {
          const text = payload.kind === 'text' ? payload.text : fallbackDictionaryText;
          await call('set_dictionary_engine', {text, engine, case_fold: true});
        }
        await call('set_free_word_mode', {on: false});
        return true;
      } catch (err) {
        console.warn('Dictionary update failed (worker)', err);
        try {
          await call('set_free_word_mode', {on: true});
        } catch {
          /* no-op */
        }
        return false;
      }
    },
    [fetchDictionaryPayload],
  );

  const applyDictionaryToWasm = useCallback(
    async (
      mod: any,
      g: any,
      engine: 'fst' | 'set' | 'dawg' | 'gaddag',
      shouldUseDict: boolean,
    ): Promise<boolean> => {
      try {
        if (!shouldUseDict) {
          mod.set_free_word_mode(g, true);
          return true;
        }
        const payload = await fetchDictionaryPayload(engine);
        if (engine === 'fst' && payload.kind === 'fst') {
          mod.set_dictionary_from_fst_bytes(g, payload.bytes, true);
        } else {
          const text = payload.kind === 'text' ? payload.text : fallbackDictionaryText;
          mod.set_dictionary_from_text_engine(g, text, engine, true);
        }
        mod.set_free_word_mode(g, false);
        return true;
      } catch (err) {
        console.warn('Dictionary update failed (wasm)', err);
        try {
          mod.set_free_word_mode(g, true);
        } catch {
          /* no-op */
        }
        return false;
      }
    },
    [fetchDictionaryPayload],
  );

  const boardWidth = useMemo(() => clamp(appliedSettings.width, 2, 30), [appliedSettings.width]);
  const boardHeight = useMemo(() => clamp(appliedSettings.height, 2, 30), [appliedSettings.height]);
  const boardDepth = useMemo(() => clamp(appliedSettings.depth, 1, 12), [appliedSettings.depth]);

  const activeMask = useMemo(() => {
    if (use3D) return null;
    return buildShapeMask(boardWidth, boardHeight, appliedSettings.shape);
  }, [use3D, boardWidth, boardHeight, appliedSettings.shape]);

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

  const appliedAdjacencyMode = useMemo(() => {
    if (useHex) return 'hex';
    if (useDiag) return 'diagonal';
    return 'orthogonal';
  }, [useHex, useDiag]);

  const tileShape = useMemo<'square' | 'hex'>(() => {
    if (use3D) return 'square';
    return appliedAdjacencyMode === 'hex' ? 'hex' : 'square';
  }, [use3D, appliedAdjacencyMode]);

  const bonusCells = useMemo<BonusCell[]>(() => {
    if (use3D) return [];
    const layout = resolveBonusPreset(
      appliedSettings.bonusPreset,
      appliedSettings.shape,
      boardWidth,
      boardHeight,
      appliedAdjacencyMode,
    );
    return layout
      .filter(cell => Number.isFinite(cell.x) && Number.isFinite(cell.y))
      .filter(cell => cell.x >= 0 && cell.x < boardWidth && cell.y >= 0 && cell.y < boardHeight)
      .filter(cell => {
        if (!activeMask) return true;
        return activeMask.has(`${cell.x},${cell.y}`);
      });
  }, [
    use3D,
    appliedSettings.bonusPreset,
    appliedSettings.shape,
    boardWidth,
    boardHeight,
    appliedAdjacencyMode,
    activeMask,
  ]);

  const bonusOverlay = useMemo(() => {
    const map = new Map<string, {label: string; tone: 'word' | 'letter'}>();
    for (const cell of bonusCells) {
      const key = `${cell.x},${cell.y}`;
      if (cell.word_mul && cell.word_mul > 1) {
        map.set(key, {label: `${cell.word_mul}W`, tone: 'word'});
        continue;
      }
      if (cell.letter_mul && cell.letter_mul > 1 && !map.has(key)) {
        map.set(key, {label: `${cell.letter_mul}L`, tone: 'letter'});
        continue;
      }
      if (cell.tags?.includes('center')) {
        map.set(key, {label: '★', tone: 'word'});
      }
    }
    return map;
  }, [bonusCells]);

  const cfg = useMemo(() => {
    const width = boardWidth;
    const height = boardHeight;
    const layoutDepth = boardDepth;

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
    boardWidth,
    boardHeight,
    boardDepth,
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
          const cfg2 = {...cfg, free_word_mode: !useDict, rng_seed: randomSeed()};
          await call('new_game', {config: cfg2, players: 2});
          await call('set_bonuses', bonusCells);
          if (cancelled) return;
          await call('set_reading_direction', {rtl});
          await call('set_stacking', {
            enabled: stackOn,
            max_height: 7,
            forbid_same: forbidSame,
            scoring: stackScoring,
          });

          const dictionaryLoaded = await applyDictionaryToWorker(call, dictEngine, useDict);
          if (!cancelled) {
            if (!dictionaryLoaded) {
              setDictReady(false);
              setDictError('Dictionary failed to load. Free-word mode enabled.');
              if (useDict) setUseDict(false);
            } else {
              setDictReady(true);
              setDictError(null);
            }
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
          const cfg2 = {...cfg, free_word_mode: !useDict, rng_seed: randomSeed()};
          const g = mod.new_game(JSON.stringify(cfg2), 2);
          mod.set_bonuses(g, JSON.stringify(bonusCells));
          mod.set_reading_direction(g, rtl);
          mod.set_stacking(g, stackOn, 7, forbidSame, stackScoring === 'sum');

          const dictionaryLoaded = await applyDictionaryToWasm(mod, g, dictEngine, useDict);
          if (!cancelled) {
            if (!dictionaryLoaded) {
              setDictReady(false);
              setDictError('Dictionary failed to load. Free-word mode enabled.');
              if (useDict) setUseDict(false);
            } else {
              setDictReady(true);
              setDictError(null);
            }
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
    dictMessagesVersion,
    ensureWorker,
    callWorker,
    ensureWasmModule,
    terminateWorker,
    bonusCells,
    applyDictionaryToWorker,
    applyDictionaryToWasm,
  ]);

  useEffect(() => {
    return () => {
      terminateWorker();
    };
  }, [terminateWorker]);

  useEffect(() => {
    if (!game) return;
    let cancelled = false;

    const refresh = async () => {
      if ('call' in game) {
        if (!useDict) {
          await game.call('set_free_word_mode', {on: true}).catch(() => {});
          if (!cancelled) {
            setDictReady(true);
            setDictError(null);
            setDictLoading(false);
          }
          return;
        }
        setDictLoading(true);
        const loaded = await applyDictionaryToWorker(game.call, dictEngine, useDict);
        if (cancelled) return;
        if (loaded) {
          setDictReady(true);
          setDictError(null);
        } else {
          setDictReady(false);
          setDictError('Dictionary failed to load. Free-word mode enabled.');
          setUseDict(false);
        }
        setDictLoading(false);
      } else {
        const {mod, g} = game;
        if (!mod || !g) return;
        if (!useDict) {
          try {
            mod.set_free_word_mode(g, true);
          } catch {
            /* ignore */
          }
          if (!cancelled) {
            setDictReady(true);
            setDictError(null);
            setDictLoading(false);
          }
          return;
        }
        setDictLoading(true);
        const loaded = await applyDictionaryToWasm(mod, g, dictEngine, useDict);
        if (cancelled) return;
        if (loaded) {
          setDictReady(true);
          setDictError(null);
        } else {
          setDictReady(false);
          setDictError('Dictionary failed to load. Free-word mode enabled.');
          setUseDict(false);
        }
        setDictLoading(false);
      }
    };

    refresh();

    return () => {
      cancelled = true;
    };
  }, [game, dictEngine, useDict, applyDictionaryToWorker, applyDictionaryToWasm]);

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

  const applySettings = useCallback((settings: SetupState): boolean => {
    const width = clamp(settings.width, 2, 30);
    const height = clamp(settings.height, 2, 30);
    const depth = clamp(settings.depth, 1, 12);
    const rackSize = clamp(settings.rackSize, 1, 14);
    const countsParsed = parseTileCounts(settings.tileCountsText, defaultTileCounts);
    if (countsParsed.error) {
      setTileCountsError(countsParsed.error);
      return false;
    }
    const scoresParsed = parseTileScores(settings.tileScoresText, defaultTileScores);
    if (scoresParsed.error) {
      setTileScoresError(scoresParsed.error);
      return false;
    }
    setTileCountsError(null);
    setTileScoresError(null);
    const nextSettings: SetupState = {
      width,
      height,
      depth,
      rackSize,
      tileCountsText: formatTileCounts(countsParsed.map),
      tileScoresText: formatTileScores(scoresParsed.map),
      shape: settings.shape,
      bonusPreset: settings.bonusPreset,
    };
    setAppliedSettings(nextSettings);
    setDraftSettings(prev => ({
      ...prev,
      width,
      height,
      depth,
      rackSize,
      tileCountsText: nextSettings.tileCountsText,
      tileScoresText: nextSettings.tileScoresText,
      shape: settings.shape,
      bonusPreset: settings.bonusPreset,
    }));
    setDictMessagesVersion(v => v + 1);
    return true;
  }, [defaultTileCounts, defaultTileScores]);

  const handleApplySettings = useCallback(() => {
    const ok = applySettings(draftSettings);
    if (!ok) return;
    setUseHex(draftAdjacencyMode === 'hex');
    setUseDiag(draftAdjacencyMode === 'diagonal');
    const enable3D = draftDimensionMode === '3d';
    setUse3D(enable3D);
    if (draftAdjacencyMode === 'hex') {
      setUseWorker(true);
    }
    if (!enable3D) {
      setZ(0);
      setInfoMessage(null);
    }
  }, [applySettings, draftSettings, draftAdjacencyMode, draftDimensionMode]);

  const dedupeMoves = useCallback((moves: GeneratedMove[]): GeneratedMove[] => {
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
  }, []);

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

  const queuePlacement = useCallback(
    (kindId: string, x: number, globalY: number, mark: string | null = null) => {
      setPending(prev => {
        if (prev.some(p => p.x === x && p.y === globalY)) return prev;
        if (!board) return prev;
        if (globalY < 0 || globalY >= board.height) return prev;
        if (!use3D && !isCellActive(x, globalY)) {
          return prev;
        }
        if (!stackOn && (board.rows[globalY]?.[x] || '').length > 0) return prev;
        const available = rackCountByKind.get(kindId) ?? 0;
        if (available <= 0) return prev;
        const used = prev.filter(p => p.kind_id === kindId).length;
        if (used >= available) {
          return prev;
        }
        return [...prev, {x, y: globalY, kind_id: kindId, mark}];
      });
    },
    [board, isCellActive, rackCountByKind, stackOn, use3D],
  );

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const api = {
      placeTile: (kindId: string, x: number, y: number, mark?: string | null) =>
        queuePlacement(kindId, x, y, mark ?? null),
      clearPending: () => setPending([]),
      pendingCount: () => pending.length,
      commitMove: () => commitMove(),
    };
    (window as any).__tileTanglePlayground = api;
    return () => {
      if ((window as any).__tileTanglePlayground === api) {
        delete (window as any).__tileTanglePlayground;
      }
    };
  }, [queuePlacement, pending, commitMove]);

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
      const lowered = message.toLowerCase();
      if (lowered.includes('unreachable')) {
        try {
          let movesJson: string;
          const limit = 50;
          if (useWorker) {
            const resp = await game.call('generate_moves', {max_len: appliedSettings.rackSize, limit});
            movesJson = String(resp.moves ?? '[]');
          } else {
            movesJson = game.mod.generate_moves(game.g, appliedSettings.rackSize, limit);
          }
          const moves: GeneratedMove[] = dedupeMoves(JSON.parse(movesJson) as GeneratedMove[]);
          if (moves.length === 0) {
            setCpuSuggestion(null);
            setCpuError('CPU has no available moves for the current rack.');
          } else {
            const best = moves.reduce((acc, mv) => {
              const total = mv.total ?? mv.score ?? 0;
              const accTotal = acc?.total ?? acc?.score ?? Number.NEGATIVE_INFINITY;
              return total > accTotal ? mv : acc;
            }, moves[0]);
            const placements: Placement[] = best.placements.map(p => ({x: p.x, y: p.y, kind_id: p.kind_id, mark: p.mark ?? null}));
            const suggestion: AiSuggestion = {
              difficulty: cpuDifficulty,
              word: best.word,
              score: best.score,
              total: best.total ?? best.score,
              rackLeave: 0,
              boardEquity: 0,
              endgamePenalty: 0,
              placements,
            };
            setCpuSuggestion(suggestion);
            setLastCpu(suggestion);
            setCpuError(null);
          }
          return;
        } catch (fallbackErr) {
          console.warn('CPU fallback from generate_moves failed', fallbackErr);
        }
      }
      if (lowered.includes('no moves available')) {
        setCpuError('CPU has no available moves for the current rack.');
      } else {
        setCpuError(`CPU hint failed: ${message}`);
      }
      setCpuSuggestion(null);
    } finally {
      setCpuThinking(false);
    }
  }, [game, useWorker, cpuDifficulty, dictReady, use3D, appliedSettings.rackSize, dedupeMoves]);

  const playCpuSuggestion = useCallback(async () => {
    if (!cpuSuggestion) return;
    const suggestion = cpuSuggestion;
    try {
      setPending([]);
      await new Promise(resolve => setTimeout(resolve, 0));
      for (const placement of suggestion.placements) {
        queuePlacement(placement.kind_id, placement.x, placement.y, placement.mark ?? null);
      }
      await new Promise(resolve => setTimeout(resolve, 0));
      await commitMove();
      setLastCpu(suggestion);
      setCpuSuggestion(null);
    } catch (err) {
      console.error('cpu autoplay failed', err);
      setCpuError(`Auto-play failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [cpuSuggestion, queuePlacement, commitMove]);

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
    if (!kindId || !board) return;
    const layerHeight = use3D ? Math.max(1, Math.floor(board.height / Math.max(1, effectiveDepth))) : board.height;
    const globalY = use3D ? y + z * layerHeight : y;
    queuePlacement(kindId, x, globalY);
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

  const handleAdjacencyModeChange = useCallback((mode: string) => {
    const next = mode as 'orthogonal' | 'diagonal' | 'hex';
    setDraftAdjacencyMode(next);
    if (next === 'hex') {
      setDraftSettings(prev => ({...prev, shape: 'diamond'}));
      setDraftDimensionMode('2d');
      setUseWorker(true);
    } else if (next === 'diagonal') {
      setDraftDimensionMode('2d');
    }
  }, []);

  const handleDimensionModeChange = useCallback((mode: string) => {
    const next = mode as '2d' | '3d';
    setDraftDimensionMode(next);
    if (next === '3d') {
      setDraftAdjacencyMode('orthogonal');
    }
  }, []);

  const boardSizeLabel = useMemo(() => {
    const size = `${appliedSettings.width}×${appliedSettings.height}`;
    if (use3D) {
      return `${size}×${Math.max(1, appliedSettings.depth)}`;
    }
    return size;
  }, [appliedSettings.width, appliedSettings.height, appliedSettings.depth, use3D]);

  const adjacencyLabel = useMemo(() => {
    if (appliedAdjacencyMode === 'hex') return 'Hex graph';
    if (appliedAdjacencyMode === 'diagonal') return 'Diagonal';
    return 'Orthogonal';
  }, [appliedAdjacencyMode]);

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
    {label: 'Automation', value: cpuLabel},
  ], [boardSizeLabel, adjacencyLabel, dimensionLabel, appliedSettings.rackSize, cpuLabel]);

  const quickPresets = useMemo<QuickPreset[]>(() => [
    {
      id: 'classic',
      title: 'Classic 15×15',
      description: 'Crossword feel with orthogonal adjacency and rack of seven.',
      onApply: () => {
        const adjacency = 'orthogonal' as const;
        const dimension = '2d' as const;
        const next: SetupState = {
          width: 15,
          height: 15,
          depth: 1,
          rackSize: 7,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'rect',
          bonusPreset: 'classic',
        };
        setDraftSettings(next);
        setDraftAdjacencyMode(adjacency);
        setDraftDimensionMode(dimension);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        const ok = applySettings(next);
        if (!ok) return;
        setUseHex(false);
        setUseDiag(false);
        setUse3D(false);
        setZ(0);
        setInfoMessage(null);
      },
    },
    {
      id: 'hex',
      title: 'Hex Garden',
      description: 'Diamond mask with full hex connectivity.',
      onApply: () => {
        const adjacency = 'hex' as const;
        const dimension = '2d' as const;
        const next: SetupState = {
          width: Math.max(7, appliedSettings.width),
          height: Math.max(7, appliedSettings.height),
          depth: 1,
          rackSize: appliedSettings.rackSize,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'diamond',
          bonusPreset: 'hex',
        };
        setDraftSettings(next);
        setDraftAdjacencyMode(adjacency);
        setDraftDimensionMode(dimension);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        const ok = applySettings(next);
        if (!ok) return;
        setUseHex(true);
        setUseDiag(false);
        setUse3D(false);
        setUseWorker(true);
        setZ(0);
        setInfoMessage(null);
      },
    },
    {
      id: 'speed',
      title: 'Sprint 11×11',
      description: 'Smaller board, six-tile rack, great for quick rounds.',
      onApply: () => {
        const adjacency = 'diagonal' as const;
        const dimension = '2d' as const;
        const next: SetupState = {
          width: 11,
          height: 11,
          depth: 1,
          rackSize: 6,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'rect',
          bonusPreset: 'classic',
        };
        setDraftSettings(next);
        setDraftAdjacencyMode(adjacency);
        setDraftDimensionMode(dimension);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        const ok = applySettings(next);
        if (!ok) return;
        setUseHex(false);
        setUseDiag(true);
        setUse3D(false);
        setZ(0);
        setInfoMessage(null);
      },
    },
    {
      id: 'stacked',
      title: '3D Tower',
      description: 'Nine-by-nine with layered play; perfect for stack mode.',
      onApply: () => {
        const adjacency = 'orthogonal' as const;
        const dimension = '3d' as const;
        const next: SetupState = {
          width: 9,
          height: 9,
          depth: Math.max(3, appliedSettings.depth),
          rackSize: appliedSettings.rackSize,
          tileCountsText: draftSettings.tileCountsText,
          tileScoresText: draftSettings.tileScoresText,
          shape: 'rect',
          bonusPreset: 'none',
        };
        setDraftSettings(next);
        setDraftAdjacencyMode(adjacency);
        setDraftDimensionMode(dimension);
        setZ(0);
        setCpuSuggestion(null);
        setLastCpu(null);
        setPending([]);
        const ok = applySettings(next);
        if (!ok) return;
        setUseHex(false);
        setUseDiag(false);
        setUse3D(true);
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
    setDraftAdjacencyMode,
    setDraftDimensionMode,
    setCpuSuggestion,
    setLastCpu,
    setPending,
    setUseHex,
    setUseDiag,
    setUse3D,
    setZ,
    setInfoMessage,
  ]);

  const themeVars = useMemo(() => buildThemeVars(palette), [palette]);

  const layerHeight = use3D && board ? Math.floor(board.height / Math.max(1, effectiveDepth)) : board?.height ?? 0;
  const cellSize = useMemo(() => {
    if (!board) return 32;
    const target = use3D ? 420 : 560;
    const computed = Math.floor(target / Math.max(board.width, 1));
    return clamp(computed, 22, 44);
  }, [board, use3D]);

  const cellGap = useMemo(() => Math.max(2, Math.min(6, Math.round(cellSize * 0.12))), [cellSize]);
  const tileFontSize = useMemo(() => Math.max(12, Math.round(cellSize * 0.62)), [cellSize]);
  const tileScoreFontSize = useMemo(() => Math.max(9, Math.round(cellSize * 0.26)), [cellSize]);
  const rackTileSize = useMemo(() => Math.max(34, cellSize + 6), [cellSize]);
  const rackFontSize = useMemo(() => Math.max(12, Math.round(rackTileSize * 0.55)), [rackTileSize]);
  const rackScoreFont = useMemo(() => Math.max(10, Math.round(rackTileSize * 0.28)), [rackTileSize]);

  const boardPalette = useMemo(
    () => ({
      boardCellBorder: palette.boardCellBorder,
      boardCellHighlight: palette.boardCellHighlight,
      boardCellBg: palette.boardCellBg,
      boardHexBorder: palette.boardHexBorder ?? palette.boardCellBorder,
    }),
    [palette],
  );

  const rackTiles: RackRowTile[] = useMemo(
    () =>
      availableRackTiles.map(({id, index}) => {
        const meta = getTileMeta(id);
        const symbol = meta?.symbol ?? id;
        const score = meta?.score;
        return {
          key: `${id}-${index}`,
          symbol,
          score,
          size: rackTileSize,
          fontSize: rackFontSize,
          scoreFontSize: rackScoreFont,
          draggable: true,
          onDragStart: (event: React.DragEvent<HTMLDivElement>) => onDragStartTile(id, event),
          testId: 'playground-rack-tile',
          dataKind: id,
        };
      }),
    [availableRackTiles, getTileMeta, onDragStartTile, rackFontSize, rackScoreFont, rackTileSize],
  );

  const moveListItems = useMemo(
    () =>
      legalMoves.map((mv, idx) => ({
        key: `${mv.word}-${idx}`,
        index: idx,
        word: mv.word,
        score: mv.total ?? mv.score,
        testId: `legal-move-${idx}`,
        actions: [
          {label: 'Highlight', onClick: () => setActiveMoveIndex(idx)},
          {label: 'Play move', onClick: () => playGeneratedMove(mv)},
        ],
      })),
    [legalMoves, playGeneratedMove, setActiveMoveIndex],
  );

  const exploreMoveButtons = useMemo(() => {
    const buttons: ButtonConfig[] = [
      {
        key: 'toggle-moves',
        label: showMoves ? 'Refresh legal moves' : 'Show legal moves',
        onClick: () => fetchMoves(),
        disabled: !game || loadingMoves || (useDict && !dictReady) || use3D,
      },
    ];
    if (showMoves) {
      buttons.push({
        key: 'hide-list',
        label: 'Hide list',
        onClick: () => {
          setShowMoves(false);
          setLegalMoves([]);
          setActiveMoveIndex(null);
        },
      });
    }
    return buttons;
  }, [dictReady, fetchMoves, game, loadingMoves, setActiveMoveIndex, showMoves, use3D, useDict]);

  const handlePreviewHover = useCallback(
    (x: number, y: number) => {
      if (!showMoves) return;
      const idx = legalMoves.findIndex(mv => mv.placements.some(p => p.x === x && p.y === y));
      if (idx >= 0) {
        setActiveMoveIndex(idx);
      }
    },
    [showMoves, legalMoves],
  );

  const playerCards = useMemo(
    () =>
      players.map(player => ({
        key: player.index,
        label: `Player ${player.index + 1}`,
        score: `${player.score} pts`,
        rack: player.rack.join(' ') || '—',
        active: player.index === activePlayer,
      })),
    [players, activePlayer],
  );

  if (!ready || !board) {
    return (
      <div className={styles.breakout}>
        <div className={styles.loading}>Loading WASM…</div>
      </div>
    );
  }

  const boardPanel = (
    <Panel title="Board" subtitle={`Turn ${turnNumber + 1} • Player ${activePlayer + 1}`} accent>
      <div className={styles.boardWrapper}>
        <PlaygroundBoard
          board={board}
          layerHeight={layerHeight}
          currentLayer={use3D ? z : 0}
          effectiveDepth={effectiveDepth}
          use3D={use3D}
          cellSize={cellSize}
          cellGap={cellGap}
          tileFontSize={tileFontSize}
          tileScoreFontSize={tileScoreFontSize}
          highlightCells={highlightCells}
          isCellActive={isCellActive}
          cellDisplay={cellDisplay}
          getTileMeta={getTileMeta}
          onDropCell={onDropCell}
          showMoves={showMoves}
          onHoverForMoves={handlePreviewHover}
          palette={boardPalette}
          tileShape={tileShape}
          renderOverlay={({x, globalY, placement}) => {
            if (placement) return null;
            const bonus = bonusOverlay.get(`${x},${globalY}`);
            if (!bonus) return null;
            const clip = tileShape === 'hex' ? HEX_POLYGON : undefined;
            return (
              <span
                className={`${styles.bonusChip} ${bonus.tone === 'word' ? styles.bonusChipWord : styles.bonusChipLetter}`}
                style={clip ? {clipPath: clip} : undefined}
                data-testid="playground-bonus"
              >
                {bonus.label}
              </span>
            );
          }}
        />
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
  );

  const stageContent = (
    <>
      {boardPanel}
      <div className={styles.stageSplit}>
        <Panel title={`Active Rack • Player ${activePlayer + 1}`} density="compact">
          <RackRow
            tiles={rackTiles}
            emptyMessage="Rack empty or all tiles placed."
            footer={
              cpuSuggestion ? (
                <div className={styles.cpuPreview}>
                  <div className={styles.cpuPreviewTitle}>CPU placements preview</div>
                  <div>{cpuSuggestion.placements.map(p => `(${p.x},${p.y})`).join(', ') || '—'}</div>
                </div>
              ) : undefined
            }
          />
        </Panel>

        <Panel title="Move Controls" density="compact">
          <ButtonRow
            buttons={[
              {
                key: 'commit',
                label: `Commit move (${pending.length})`,
                onClick: commitMove,
                disabled: pending.length === 0,
              },
              {
                key: 'reset',
                label: 'Reset pending',
                onClick: handlePendingReset,
                disabled: pending.length === 0,
              },
            ]}
          />
          <ButtonRow
            buttons={[
              {
                key: 'undo',
                label: 'Undo',
                onClick: undoMove,
                disabled: !game,
                testId: 'playground-undo',
              },
              {
                key: 'redo',
                label: 'Redo',
                onClick: redoMove,
                disabled: !game,
                testId: 'playground-redo',
              },
              {
                key: showMoves ? 'hide-moves' : 'show-moves',
                label: showMoves ? 'Hide legal moves' : 'Show legal moves',
                onClick: () => {
                  if (showMoves) {
                    setShowMoves(false);
                    setLegalMoves([]);
                    setActiveMoveIndex(null);
                  } else {
                    fetchMoves();
                  }
                },
                disabled: !game || loadingMoves || (useDict && !dictReady) || use3D,
              },
            ]}
          />
          {showMoves && !loadingMoves && (
            <ButtonRow
              buttons={[
                {
                  key: 'refresh',
                  label: 'Refresh legal moves',
                  onClick: () => fetchMoves(),
                  disabled: !game || loadingMoves || (useDict && !dictReady) || use3D,
                },
                {
                  key: 'hide-list',
                  label: 'Hide list',
                  onClick: () => {
                    setShowMoves(false);
                    setLegalMoves([]);
                    setActiveMoveIndex(null);
                  },
                },
              ]}
            />
          )}
          {loadingMoves && <div className={styles.helperText}>Loading legal moves…</div>}
        </Panel>
      </div>
    </>
  );

  const sidebarContent = (
    <>
      <BoardSetupPanel draft={draftSettings} onDraftChange={handleDraftSettingsChange} onApply={handleApplySettings} />
      <LanguagePanel
        useDict={useDict}
        onUseDictChange={setUseDict}
        dictEngine={dictEngine}
        onDictEngineChange={setDictEngine}
        useAnagram={useAnagram}
        onUseAnagramChange={setUseAnagram}
        rtl={rtl}
        onRtlChange={setRtl}
        dictLoading={dictLoading}
      />
      <StackingPanel
        stackOn={stackOn}
        onStackOnChange={setStackOn}
        stackScoring={stackScoring}
        onStackScoringChange={setStackScoring}
        forbidSame={forbidSame}
        onForbidSameChange={setForbidSame}
      />
    </>
  );

  const rightRailContent = (
    <>
      <Panel title="Turn Tracker" subtitle="Scoreboard and bag" density="compact">
        <PlayerList players={playerCards} emptyMessage="Loading players…" />
        <div className={styles.helperText}>
          Bag: {bagSummary.total} tiles{bagSummary.total > 0 && bagPreview ? ` • ${bagPreview}` : ''}
        </div>
      </Panel>

      <Panel title="Playground Modes" subtitle="Switch adjacency and runtime modes." density="compact">
        <div className={styles.modeGroup}>
          <div className={styles.modeLabel}>Adjacency</div>
          <SegmentedControl
            name="Adjacency"
            value={draftAdjacencyMode}
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
            value={draftDimensionMode}
            onChange={handleDimensionModeChange}
            options={[
              {value: '2d', label: '2D'},
              {value: '3d', label: '3D', hint: 'Stacked'},
            ]}
          />
        </div>
        <ToggleField label="Run heavy work in a Web Worker" checked={useWorker} onChange={setUseWorker} />
        {draftAdjacencyMode === 'hex' && (
          <div className={styles.helperText}>Hex adjacency uses staggered rows with three axes (E, NE, SE).</div>
        )}
        {draftAdjacencyMode === 'diagonal' && (
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
        <ButtonRow
          buttons={[
            {
              key: 'cpu-hint',
              label: cpuThinking ? 'Thinking…' : 'CPU hint',
              onClick: requestCpuHint,
              disabled: !game || cpuDifficulty === 'off' || cpuThinking || !dictReady || use3D,
            },
            {
              key: 'cpu-play',
              label: 'Play as CPU',
              onClick: playCpuSuggestion,
              disabled: !game || cpuSuggestion == null || cpuThinking || use3D,
              testId: 'cpu-play-button',
            },
          ]}
        />
        {cpuThinking && <div className={styles.helperText}>Computing best move…</div>}
        {cpuSuggestion && (
          <CpuHintSummary
            tone="active"
            title={<>CPU ({cpuSuggestion.difficulty}) suggests</>}
            word={cpuSuggestion.word}
            total={<>{cpuSuggestion.total} pts</>}
            meta={
              <>Raw {cpuSuggestion.score}, leave {cpuSuggestion.rackLeave}, equity {cpuSuggestion.boardEquity}</>
            }
          />
        )}
        {!cpuSuggestion && lastCpu && (
          <CpuHintSummary
            tone="muted"
            title={<>Last hint ({lastCpu.difficulty})</>}
            word={lastCpu.word}
            total={<>{lastCpu.total} pts</>}
            meta={
              <>Raw {lastCpu.score}, leave {lastCpu.rackLeave}, equity {lastCpu.boardEquity}</>
            }
          />
        )}
        {cpuError && <div className={styles.errorText}>{cpuError}</div>}
      </Panel>

      <Panel
        title="Explore Moves"
        subtitle="Generate legal plays for the current rack."
        density="compact"
        accent={showMoves}
      >
        <ButtonRow buttons={exploreMoveButtons} />
        {showMoves && (
          loadingMoves ? (
            <div className={styles.helperText}>Loading legal moves…</div>
          ) : (
            <MoveList
              items={moveListItems}
              emptyMessage="No moves available for the current rack."
            />
          )
        )}
      </Panel>

      <Panel title="Snapshots & Log" subtitle="Export state or inspect history." density="compact">
        <textarea
          value={snapshotText}
          onChange={e => setSnapshotText(e.target.value)}
          rows={5}
          className={styles.snapshotArea}
          placeholder="Click Save snapshot to capture the current game state"
        />
        <ButtonRow
          buttons={[
            {key: 'save', label: 'Save snapshot', onClick: exportSnapshot, disabled: !game},
            {
              key: 'load',
              label: 'Load snapshot',
              onClick: importSnapshot,
              disabled: !game || snapshotText.trim() === '',
            },
            {key: 'log', label: 'Show event log', onClick: fetchEventLog, disabled: !game},
          ]}
        />
        {eventLogText && <pre className={styles.logViewer}>{eventLogText}</pre>}
      </Panel>
    </>
  );

  const heroNode = (
    <PlaygroundHero
      eyebrow="TileTangle Playground"
      title="Design. Experiment. Solve."
      description="Tune adjacency, stack rules, and automation to watch the engine reshape every move in real time."
      stats={heroStats}
      rightSlot={(
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
      )}
    />
  );

  const alerts: AlertItem[] = [];
  if (errorMessage) {
    alerts.push({id: 'error', kind: 'error', message: errorMessage});
  }
  if (dictError) {
    alerts.push({id: 'dict', kind: 'warning', message: dictError});
  }
  if (cpuError) {
    alerts.push({id: 'cpu', kind: 'warning', message: cpuError});
  }
  if (infoMessage) {
    alerts.push({id: 'info', kind: 'info', message: infoMessage});
  }

  const alertsNode = <AlertStack alerts={alerts} />;

  return (
    <PlaygroundShell
      themeVars={themeVars}
      hero={heroNode}
      alerts={alertsNode}
      sidebar={sidebarContent}
      main={stageContent}
      rightRail={rightRailContent}
    />
  );
}
