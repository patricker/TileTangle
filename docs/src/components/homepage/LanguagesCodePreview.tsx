import React from 'react';
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CodeBlock from '@theme/CodeBlock';

export default function LanguagesCodePreview(): JSX.Element {
  return (
    <div aria-label="Language snippets preview" style={{display: 'inline-block', textAlign: 'left', maxWidth: 360}}>
      <Tabs>
        <TabItem value="js" label="Web / JS" default>
          <CodeBlock language="ts">{
`import init, { new_game, get_board } from '@tiletangle/engine-wasm';
await init();
const cfg = { board_layout: { width: 15, height: 15 }, rack_size: 7, ruleset_id: 'cross', free_word_mode: true, tileset: { tile_kinds: [{ id: 'A', score: 1 }] }, tile_counts: { A: 50 } };
const game = new_game(JSON.stringify(cfg), 2);
const board = JSON.parse(get_board(game));
console.log(board.width, board.height);
`}
          </CodeBlock>
        </TabItem>
        <TabItem value="py" label="Python">
          <CodeBlock language="python">{
`from tiletangle import Game
import json
cfg = {"board_layout": {"width": 15, "height": 15}, "rack_size": 7, "ruleset_id": "cross", "free_word_mode": True}
g = Game(json.dumps(cfg), 2)
print(g.get_board_json())`
          }
          </CodeBlock>
        </TabItem>
        <TabItem value="rs" label="Rust">
          <CodeBlock language="rust">{
`use tiletangle_engine as tt;
let cfg = tt::Config { board_layout: tt::BoardLayout { width: 15, height: 15 }, rack_size: 7, ..Default::default() };
let mut g = tt::Game::new(cfg, 2);
let b = g.board();
println!("{}x{}", b.width, b.height);`
          }
          </CodeBlock>
        </TabItem>
      </Tabs>
    </div>
  );
}
