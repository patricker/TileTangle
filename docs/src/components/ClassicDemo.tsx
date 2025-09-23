import React, {useCallback, useEffect, useMemo, useState} from 'react';
import useBaseUrl from '@docusaurus/useBaseUrl';
import {useColorMode} from '@docusaurus/theme-common';

import {classicBonuses, classicTilesets} from './demoUtils';
import PlaygroundBoard from './playground/PlaygroundBoard';
import PlaygroundHero from './playground/PlaygroundHero';
import PlaygroundShell from './playground/PlaygroundShell';
import AlertStack, {type AlertItem} from './playground/AlertStack';
import {buildThemeVars, getPlaygroundPalette} from './playground/theme';
import {ButtonRow, MoveList, Panel, RackRow, ToggleField, type RackRowTile} from './playground/ui';
import {StackingControls} from './playground/panels';
import type {BoardJson} from './playground/types';
import styles from './PlaygroundLayout.module.css';
import {useWorkerMessenger} from './playground/useWorkerMessenger';
import {randomSeed} from './playground/random';

type Placement = {x: number; y: number; kind_id: string; mark?: string | null};
type Hint = {placements: Placement[]; score: number; word: string};
type WorkerGame = {call: (action: string, payload?: any) => Promise<any>};

const BOARD_WIDTH = 15;
const BOARD_HEIGHT = 15;
const MAX_HINTS = 5;

export default function ClassicDemo({compact}: {compact?: boolean}): JSX.Element {
  const {colorMode} = useColorMode();
  const palette = useMemo(() => getPlaygroundPalette(colorMode as 'light' | 'dark'), [colorMode]);
  const themeVars = useMemo(() => buildThemeVars(palette), [palette]);

  const classic = useMemo(() => classicTilesets(), []);
  const bonusLayout = useMemo(() => classicBonuses(), []);
  const bonusOverlay = useMemo(() => {
    const map = new Map<string, {label: string; tone: 'word' | 'letter'}>();
    for (const cell of bonusLayout) {
      const key = `${cell.x},${cell.y}`;
      if (cell.word_mul && cell.word_mul > 1) {
        map.set(key, {label: `${cell.word_mul}W`, tone: 'word'});
        continue;
      }
      if (cell.letter_mul && cell.letter_mul > 1 && !map.has(key)) {
        map.set(key, {label: `${cell.letter_mul}L`, tone: 'letter'});
      }
    }
    return map;
  }, [bonusLayout]);

  const [game, setGame] = useState<WorkerGame | null>(null);
  const [board, setBoard] = useState<BoardJson | null>(null);
  const [rack, setRack] = useState<{kind_id: string; mark?: string | null}[]>([]);
  const [scores, setScores] = useState<number[]>([]);
  const [activePlayer, setActivePlayer] = useState(0);
  const [pending, setPending] = useState<Placement[]>([]);
  const [exchangeSel, setExchangeSel] = useState<Set<number>>(new Set());
  const [useDict, setUseDict] = useState(false);
  const [rtl, setRtl] = useState(false);
  const [stackOn, setStackOn] = useState(false);
  const [stackScoring, setStackScoring] = useState<'top' | 'sum'>('top');
  const [forbidSame, setForbidSame] = useState(true);
  const [showHints, setShowHints] = useState(false);
  const [hints, setHints] = useState<Hint[]>([]);
  const [activeHintIndex, setActiveHintIndex] = useState<number | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [infoMessage, setInfoMessage] = useState<string | null>(null);

  const {ensureWorker, callWorker, terminateWorker} = useWorkerMessenger();
  const fstUrl = useBaseUrl('dictionaries/TWL06.fst');
  const txtUrl = useBaseUrl('dictionaries/TWL06.txt');

  const cfg = useMemo(() => ({
    tileset: {tile_kinds: classic.tile_kinds},
    rack_size: 7,
    board_layout: {width: BOARD_WIDTH, height: BOARD_HEIGHT},
    ruleset_id: 'cross',
    dictionary_id: 'en',
    rng_seed: 1,
    tile_counts: classic.tile_counts,
    free_word_mode: !useDict,
  }) as any,
  [classic.tile_kinds, classic.tile_counts, useDict]);

  const tileMetaMap = useMemo(() => {
    const map = new Map<string, {symbol: string; score: number; isBlank: boolean}>();
    for (const kind of classic.tile_kinds) {
      map.set(kind.id, {
        symbol: kind.symbol || kind.id,
        score: kind.score,
        isBlank: !!kind.is_blank,
      });
    }
    return map;
  }, [classic.tile_kinds]);

  const getTileMeta = useCallback(
    (id: string) => tileMetaMap.get(id),
    [tileMetaMap],
  );

  const syncState = useCallback(async (call: WorkerGame['call']) => {
    const [boardResp, rackResp, scoresResp, snapshotResp] = await Promise.all([
      call('get_board'),
      call('get_rack'),
      call('get_scores'),
      call('snapshot_json'),
    ]);
    const nextBoard = JSON.parse(boardResp.board as string) as BoardJson;
    const nextScores = JSON.parse(scoresResp.scores as string) as number[];
    setBoard(nextBoard);
    setRack(JSON.parse(rackResp.rack as string));
    setScores(nextScores);
    try {
      const snapshot = JSON.parse(String(snapshotResp.snapshot ?? '{}')) as {to_move?: number};
      const toMove = typeof snapshot.to_move === 'number' ? snapshot.to_move : 0;
      const playerCount = Math.max(1, nextScores.length);
      setActivePlayer(playerCount > 0 ? toMove % playerCount : 0);
    } catch (err) {
      console.warn('Failed to parse snapshot for active player', err);
      setActivePlayer(0);
    }
    setPending([]);
    setExchangeSel(new Set());
  }, []);

  useEffect(() => {
    let cancelled = false;

    const worker = ensureWorker();
    if (!worker) {
      setErrorMessage('Web Workers are not available in this environment.');
      return () => undefined;
    }

    const call: WorkerGame['call'] = async (action, payload) => {
      ensureWorker();
      return callWorker(action, payload);
    };

    const loadDictionary = async () => {
      try {
        const fst = await fetch(fstUrl);
        if (fst.ok) {
          const buf = new Uint8Array(await fst.arrayBuffer());
          await call('set_dictionary_from_fst_bytes', {bytes: buf, case_fold: true});
          return;
        }
      } catch (err) {
        console.warn('FST dictionary fetch failed', err);
      }
      try {
        const txtResp = await fetch(txtUrl);
        if (txtResp.ok) {
          const text = await txtResp.text();
          await call('set_dictionary_from_text', {text, case_fold: true});
        }
      } catch (err) {
        console.warn('Dictionary text fetch failed', err);
      }
    };

    const initialise = async () => {
      try {
        setErrorMessage(null);
        setInfoMessage('Initialising classic demo…');
        const nextSeed = randomSeed();
        const config = {...cfg, rng_seed: nextSeed || 1};
        await call('new_game', {config, players: 2});
        await call('set_bonuses', bonusLayout);
        await call('set_reading_direction', {rtl});
        await call('set_stacking', {
          enabled: stackOn,
          max_height: 7,
          forbid_same: forbidSame,
          scoring: stackScoring,
        });
        await call('set_free_word_mode', {on: !useDict});
        if (useDict) {
          await loadDictionary();
        }
        if (cancelled) return;
        await syncState(call);
        if (cancelled) return;
        setGame({call});
        setInfoMessage(null);
      } catch (err) {
        console.error('Classic demo initialisation failed', err);
        if (!cancelled) {
          setErrorMessage('Failed to initialise the classic demo. See console for details.');
        }
      }
    };

    initialise();

    return () => {
      cancelled = true;
      terminateWorker();
      setGame(null);
      setBoard(null);
      setRack([]);
      setHints([]);
    };
  }, [bonusLayout, cfg, rtl, stackOn, stackScoring, forbidSame, useDict, syncState, ensureWorker, callWorker, terminateWorker, fstUrl, txtUrl]);

  const updateFromGame = useCallback(async () => {
    if (!game) return;
    await syncState(game.call);
  }, [game, syncState]);

  const queuePlacement = useCallback(
    (placement: Placement) => {
      setPending(prev => {
        if (prev.some(p => p.x === placement.x && p.y === placement.y)) {
          return prev;
        }
        return [...prev, placement];
      });
    },
    [],
  );

  const commitMove = useCallback(async () => {
    if (!game || pending.length === 0) return;
    try {
      await game.call('play_move', {placements: pending});
      await updateFromGame();
    } catch (err) {
      console.error('commit failed', err);
      setErrorMessage(`Commit failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, pending, updateFromGame]);

  const passTurn = useCallback(async () => {
    if (!game) return;
    try {
      await game.call('pass_turn');
      await updateFromGame();
    } catch (err) {
      console.error('pass failed', err);
      setErrorMessage(`Pass failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, updateFromGame]);

  const exchangeTiles = useCallback(async () => {
    if (!game || exchangeSel.size === 0) return;
    try {
      const kinds = Array.from(exchangeSel).map(index => rack[index]?.kind_id).filter(Boolean);
      await game.call('exchange_tiles', {kinds});
      await updateFromGame();
      setExchangeSel(new Set());
    } catch (err) {
      console.error('exchange failed', err);
      setErrorMessage(`Exchange failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [exchangeSel, game, rack, updateFromGame]);

  const undo = useCallback(async () => {
    if (!game) return;
    try {
      await game.call('undo');
      await updateFromGame();
    } catch (err) {
      console.error('undo failed', err);
      setErrorMessage(`Undo failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, updateFromGame]);

  const redo = useCallback(async () => {
    if (!game) return;
    try {
      await game.call('redo');
      await updateFromGame();
    } catch (err) {
      console.error('redo failed', err);
      setErrorMessage(`Redo failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, updateFromGame]);

  const aiMove = useCallback(async () => {
    if (!game) return;
    try {
      const {moves} = await game.call('generate_moves', {max_len: 15, limit: 1});
      const parsed = JSON.parse(moves as string) as Hint[];
      if (parsed.length === 0) {
        setInfoMessage('AI could not find a move with the current rack.');
        return;
      }
      await game.call('play_move', {placements: parsed[0].placements});
      await updateFromGame();
    } catch (err) {
      console.error('AI move failed', err);
      setErrorMessage(`AI move failed: ${(err as Error).message ?? String(err)}`);
    }
  }, [game, updateFromGame]);

  useEffect(() => {
    let cancelled = false;
    const fetchHints = async () => {
      if (!game || !showHints) {
        if (!cancelled) {
          setHints([]);
          setActiveHintIndex(null);
        }
        return;
      }
      try {
        const {moves} = await game.call('generate_moves', {max_len: 15, limit: MAX_HINTS});
        if (cancelled) return;
        const parsed = JSON.parse(moves as string) as Hint[];
        setHints(parsed);
        setActiveHintIndex(parsed.length > 0 ? 0 : null);
      } catch (err) {
        console.error('Hint generation failed', err);
        if (!cancelled) setHints([]);
      }
    };
    fetchHints();
    return () => {
      cancelled = true;
    };
  }, [game, showHints, board]);

  const hintOverlay = useMemo(() => {
    if (!showHints) return new Map<string, number>();
    const map = new Map<string, number>();
    hints.forEach((hint, index) => {
      hint.placements.forEach(placement => {
        const key = `${placement.x},${placement.y}`;
        if (!map.has(key)) {
          map.set(key, index + 1);
        }
      });
    });
    return map;
  }, [hints, showHints]);

  const highlightCells = useMemo(() => {
    if (!showHints || activeHintIndex == null) return new Set<string>();
    const hint = hints[activeHintIndex];
    if (!hint) return new Set<string>();
    return new Set(hint.placements.map(p => `${p.x},${p.y}`));
  }, [activeHintIndex, hints, showHints]);

  const layerHeight = board?.height ?? BOARD_HEIGHT;
  const effectiveDepth = 1;
  const cellSize = useMemo(() => {
    if (!board) return 32;
    return Math.max(26, Math.min(42, Math.floor(520 / Math.max(board.width, board.height))));
  }, [board]);
  const cellGap = useMemo(() => Math.max(2, Math.round(cellSize * 0.12)), [cellSize]);
  const tileFontSize = useMemo(() => Math.max(12, Math.round(cellSize * 0.62)), [cellSize]);
  const tileScoreFontSize = useMemo(() => Math.max(9, Math.round(cellSize * 0.26)), [cellSize]);
  const rackTileSize = useMemo(() => Math.max(36, cellSize + 8), [cellSize]);
  const rackFontSize = useMemo(() => Math.max(12, Math.round(rackTileSize * 0.55)), [rackTileSize]);
  const rackScoreFont = useMemo(() => Math.max(10, Math.round(rackTileSize * 0.28)), [rackTileSize]);

  const isCellActive = useCallback(() => true, []);

  const cellDisplay = useCallback(
    (x: number, viewY: number) => {
      const placement = pending.find(p => p.x === x && p.y === viewY);
      if (placement) {
        return placement.mark && placement.mark.length > 0 ? placement.mark : placement.kind_id;
      }
      if (!board) return '';
      return board.rows[viewY]?.[x] ?? '';
    },
    [board, pending],
  );

  const handleHoverForHints = useCallback(
    (x: number, y: number) => {
      if (!showHints) return;
      const idx = hints.findIndex(hint => hint.placements.some(p => p.x === x && p.y === y));
      setActiveHintIndex(idx >= 0 ? idx : null);
    },
    [hints, showHints],
  );

  const onDropCell = useCallback(
    (x: number, y: number, ev: React.DragEvent<HTMLDivElement>) => {
      ev.preventDefault();
      const payload = ev.dataTransfer.getData('text/plain');
      if (!payload) return;
      const [kindId, blankFlag] = payload.split(':');
      const meta = getTileMeta(kindId);
      let mark: string | null = null;
      if (blankFlag === '1') {
        const response = window.prompt('Blank tile: enter symbol', 'A');
        mark = (response || '').trim().toUpperCase() || null;
      } else if (meta?.isBlank) {
        const response = window.prompt('Blank tile: enter symbol', meta.symbol);
        mark = (response || '').trim().toUpperCase() || null;
      }
      queuePlacement({x, y, kind_id: kindId, mark});
    },
    [getTileMeta, queuePlacement],
  );

  const onDragStartTile = useCallback(
    (tile: {kind_id: string; mark?: string | null}, ev: React.DragEvent<HTMLDivElement>) => {
      const meta = getTileMeta(tile.kind_id);
      const isBlank = meta?.isBlank ? '1' : '0';
      ev.dataTransfer.setData('text/plain', `${tile.kind_id}:${isBlank}`);
    },
    [getTileMeta],
  );

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const api = {
      placeTile: (kindId: string, x: number, y: number, mark?: string | null) =>
        queuePlacement({x, y, kind_id: kindId, mark: mark ?? null}),
      clearPending: () => setPending([]),
      commitMove: () => commitMove(),
    };
    (window as any).__classicDemo = api;
    return () => {
      if ((window as any).__classicDemo === api) {
        delete (window as any).__classicDemo;
      }
    };
  }, [queuePlacement, commitMove]);

  const heroStats = useMemo(
    () => [
      {label: 'Turn', value: activePlayer === 0 ? 'Player 1' : 'Player 2'},
      {label: 'Player 1', value: `${scores[0] ?? 0} pts`},
      {label: 'Player 2', value: `${scores[1] ?? 0} pts`},
      {label: 'Rack', value: '7 tiles'},
    ],
    [scores, activePlayer],
  );

  const rackTiles: RackRowTile[] = useMemo(
    () =>
      rack.map((tile, index) => {
        const meta = getTileMeta(tile.kind_id);
        const symbol = tile.mark && tile.mark.length > 0 ? tile.mark : meta?.symbol ?? tile.kind_id;
        const score = meta?.score ?? 0;
        const selected = exchangeSel.has(index);
        return {
          key: `${tile.kind_id}-${index}`,
          symbol,
          score,
          size: rackTileSize,
          fontSize: rackFontSize,
          scoreFontSize: rackScoreFont,
          draggable: true,
          onDragStart: (ev: React.DragEvent<HTMLDivElement>) => onDragStartTile(tile, ev),
          onClick: () => {
            const next = new Set(exchangeSel);
            selected ? next.delete(index) : next.add(index);
            setExchangeSel(next);
          },
          ariaPressed: selected,
          style: {
            borderColor: selected ? palette.accentBorder : palette.rackTileBorder,
            background: selected ? palette.rackTileHighlight : palette.rackTileBg,
          },
          dataKind: tile.kind_id,
          testId: 'classic-rack-tile',
        };
      }),
    [exchangeSel, getTileMeta, onDragStartTile, palette.accentBorder, palette.rackTileBg, palette.rackTileBorder, palette.rackTileHighlight, rack, rackFontSize, rackScoreFont, rackTileSize, setExchangeSel],
  );

  const playerCards = useMemo(
    () => [
      {
        key: 0,
        label: 'Player 1',
        score: `${scores[0] ?? 0} pts`,
        active: activePlayer === 0,
      },
      {
        key: 1,
        label: 'Player 2',
        score: `${scores[1] ?? 0} pts`,
        active: activePlayer === 1,
      },
    ],
    [scores, activePlayer],
  );

  const hintItems = useMemo(
    () =>
      hints.map((hint, index) => ({
        key: `${hint.word}-${index}`,
        index,
        word: hint.word,
        score: hint.score,
        testId: 'classic-hint-card',
        actions: [
          {label: 'Highlight', onClick: () => setActiveHintIndex(index)},
          {
            label: 'Play move',
            onClick: async () => {
              if (!game) return;
              try {
                await game.call('play_move', {placements: hint.placements});
                await updateFromGame();
              } catch (err) {
                console.error('play hint failed', err);
                setErrorMessage(`Playing hint failed: ${(err as Error).message ?? String(err)}`);
              }
            },
          },
        ],
      })),
    [game, hints, setErrorMessage, setActiveHintIndex, updateFromGame],
  );

  const hintList = showHints ? (
    <MoveList
      items={hintItems}
      emptyMessage={<div className={styles.helperText}>Enable hints to preview generated moves.</div>}
    />
  ) : (
    <div className={styles.helperText}>Enable hints to preview generated moves.</div>
  );

  const alerts: AlertItem[] = [];
  if (errorMessage) {
    alerts.push({id: 'error', kind: 'error', message: errorMessage});
  }
  if (infoMessage) {
    alerts.push({id: 'info', kind: 'info', message: infoMessage});
  }

  const alertsNode = <AlertStack alerts={alerts} />;

  if (!board) {
    return (
      <div className={styles.breakout}>
        <div className={styles.loading}>Loading classic demo…</div>
      </div>
    );
  }

  const sidebarContent = (
    <>
      <Panel
        title="Game Setup"
        subtitle="Toggle validation and stacking rules."
        density="compact"
      >
        <ToggleField label="Dictionary checks" checked={useDict} onChange={setUseDict} />
        <ToggleField label="RTL reading direction" checked={rtl} onChange={setRtl} />
        <StackingControls
          stackOn={stackOn}
          onStackOnChange={setStackOn}
          stackScoring={stackScoring}
          onStackScoringChange={setStackScoring}
          forbidSame={forbidSame}
          onForbidSameChange={setForbidSame}
        />
      </Panel>

      <Panel title="Automation" subtitle="Let the engine explore." density="compact">
        <ButtonRow
          buttons={[
            {key: 'ai-move', label: 'AI move', onClick: aiMove, disabled: !game},
            {
              key: 'toggle-hints',
              label: showHints ? 'Hide hints' : 'Show hints',
              onClick: () => setShowHints(v => !v),
            },
          ]}
        />
        <div className={styles.helperText}>
          AI moves use the same solver that powers the Playground presets.
        </div>
      </Panel>
    </>
  );

  const stageContent = (
    <>
      <Panel
        title="Classic board"
        subtitle={`Scores: ${(scores[0] ?? 0)} – ${(scores[1] ?? 0)}`}
        accent
      >
        <div className={styles.boardWrapper}>
          <PlaygroundBoard
            board={board}
            layerHeight={layerHeight}
            currentLayer={0}
            effectiveDepth={effectiveDepth}
            use3D={false}
            cellSize={cellSize}
            cellGap={cellGap}
            tileFontSize={tileFontSize}
            tileScoreFontSize={tileScoreFontSize}
            highlightCells={highlightCells}
            isCellActive={isCellActive}
            cellDisplay={cellDisplay}
            getTileMeta={id => {
              const meta = getTileMeta(id);
              return meta ? {symbol: meta.symbol, score: meta.score} : undefined;
            }}
            onDropCell={onDropCell}
            showMoves={showHints}
            onHoverForMoves={handleHoverForHints}
            palette={{
              boardCellBorder: palette.boardCellBorder,
              boardCellHighlight: palette.boardCellHighlight,
              boardCellBg: palette.boardCellBg,
            }}
            renderOverlay={({x, globalY, placement}) => {
              if (placement) return null;
              const key = `${x},${globalY}`;
              const bonus = bonusOverlay.get(key);
              const label = showHints ? hintOverlay.get(key) : null;
              if (!bonus && !label) return null;
              return (
                <>
                  {bonus && (
                    <span
                      className={`${styles.bonusChip} ${bonus.tone === 'word' ? styles.bonusChipWord : styles.bonusChipLetter}`}
                    >
                      {bonus.label}
                    </span>
                  )}
                  {label && (
                    <span className={styles.hintBadge} data-testid="classic-hint-badge">
                      {label}
                    </span>
                  )}
                </>
              );
            }}
          />
        </div>
      </Panel>

      <Panel title="Turn tracker" density="compact">
        <div className={styles.playerList}>
          {playerCards.map(card => (
            <div
              key={card.key}
              className={`${styles.playerCard} ${card.active ? styles.playerCardActive : ''}`}
              data-active={card.active ? '1' : '0'}
            >
              <div className={styles.playerCardHeader}>
                {card.label}
                {card.active && <span className={styles.activeBadge}>Your turn</span>}
              </div>
              <div className={styles.playerScore}>{card.score}</div>
            </div>
          ))}
        </div>
      </Panel>

      <Panel title="Rack" density="compact">
        <RackRow tiles={rackTiles} emptyMessage="Rack empty" />
      </Panel>
    </>
  );

  const rightRailContent = (
    <>
      <Panel title="Move controls" density="compact">
        <ButtonRow
          buttons={[
            {
              key: 'commit',
              label: `Commit (${pending.length})`,
              onClick: commitMove,
              disabled: !game || pending.length === 0,
            },
            {
              key: 'reset',
              label: 'Reset pending',
              onClick: () => setPending([]),
              disabled: pending.length === 0,
            },
          ]}
        />
        <ButtonRow
          buttons={[
            {key: 'undo', label: 'Undo', onClick: undo, disabled: !game},
            {key: 'redo', label: 'Redo', onClick: redo, disabled: !game},
            {key: 'pass', label: 'Pass', onClick: passTurn, disabled: !game},
          ]}
        />
        <ButtonRow
          buttons={[
            {
              key: 'exchange',
              label: `Exchange (${exchangeSel.size})`,
              onClick: exchangeTiles,
              disabled: !game || exchangeSel.size === 0,
            },
          ]}
        />
      </Panel>

      <Panel title="Hints" subtitle="Top generated plays" density="compact">
        {hintList}
      </Panel>
    </>
  );

  const heroNode = (
    <PlaygroundHero
      eyebrow="Classic Crossword Demo"
      title="Familiar rules, flexible engine."
      description="Drag tiles, exchange racks, and exercise the same WASM engine that powers the full Playground — now styled for the classic 15×15 experience."
      stats={heroStats}
    />
  );

  const compactMode = compact ?? true;

  return (
    <PlaygroundShell
      themeVars={themeVars}
      hero={heroNode}
      alerts={alertsNode}
      sidebar={sidebarContent}
      main={stageContent}
      rightRail={rightRailContent}
      compact={compactMode}
    />
  );
}
