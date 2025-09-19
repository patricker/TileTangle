import assert from 'node:assert/strict';
import {renderToStaticMarkup} from 'react-dom/server';
import React from 'react';
import {createRequire} from 'module';

async function main() {
  const require = createRequire(import.meta.url);
  (require as any).extensions['.css'] = () => ({
    exports: {},
  });

  const {
    ButtonRow,
    MoveList,
    RackRow,
    ToggleField,
    PlayerList,
    CpuHintSummary,
  } = await import('../src/components/playground/ui');

  const noop = () => {};

  function assertContains(markup: string, needle: string, message: string) {
    assert.ok(markup.includes(needle), message);
  }

  const buttonsMarkup = renderToStaticMarkup(
    <ButtonRow
      buttons={[
        {label: 'One', onClick: noop},
        {label: 'Two', disabled: true},
      ]}
    />,
  );
  assertContains(buttonsMarkup, 'One', 'ButtonRow renders first button');
  assertContains(buttonsMarkup, 'Two', 'ButtonRow renders second button');

  const rackMarkup = renderToStaticMarkup(
    <RackRow
      tiles={[
        {
          key: 'A0',
          symbol: 'A',
          score: 1,
          size: 40,
          fontSize: 16,
          scoreFontSize: 10,
        },
      ]}
    />,
  );
  assertContains(rackMarkup, 'A', 'RackRow renders tile symbol');

  const moveMarkup = renderToStaticMarkup(
    <MoveList
      items={[
        {
          key: 'MOVE-0',
          index: 0,
          word: 'WORD',
          score: 30,
        },
      ]}
    />,
  );
  assertContains(moveMarkup, 'WORD', 'MoveList renders move word');
  assertContains(moveMarkup, '30 pts', 'MoveList renders move score');

  const toggleMarkup = renderToStaticMarkup(
    <ToggleField label="Example toggle" checked onChange={noop} />,
  );
  assertContains(toggleMarkup, 'Example toggle', 'ToggleField renders label');

  const playerMarkup = renderToStaticMarkup(
    <PlayerList
      players={[
        {key: 'p1', label: 'Player 1', score: '10 pts', rack: 'ABC', active: true},
        {key: 'p2', label: 'Player 2', score: '5 pts'},
      ]}
    />,
  );
  assertContains(playerMarkup, 'Player 1', 'PlayerList renders first player');
  assertContains(playerMarkup, 'Player 2', 'PlayerList renders second player');

  const cpuMarkup = renderToStaticMarkup(
    <CpuHintSummary
      title="CPU"
      word="HELLO"
      total="42 pts"
      meta="Raw 20, leave 5, equity 10"
    />,
  );
  assertContains(cpuMarkup, 'HELLO', 'CpuHintSummary renders word');
  assertContains(cpuMarkup, '42 pts', 'CpuHintSummary renders total');

  const emptyPlayerMarkup = renderToStaticMarkup(
    <PlayerList players={[]} emptyMessage="Loading players…" />,
  );
  assertContains(emptyPlayerMarkup, 'Loading players…', 'PlayerList renders empty message');

  const emptyMovesMarkup = renderToStaticMarkup(
    <MoveList items={[]} emptyMessage="No moves" />,
  );
  assertContains(emptyMovesMarkup, 'No moves', 'MoveList renders empty message');

  console.log('UI component smoke checks passed');
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
