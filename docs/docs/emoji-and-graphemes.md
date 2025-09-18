---
sidebar_position: 14
---

# Emoji & Graphemes

import Playground from '@site/src/components/Playground';

Tile symbols can be multi-codepoint grapheme clusters (emoji with skin-tone modifiers, ZWJ families, or
digraph tiles). TileTangle stores symbols as grapheme clusters end-to-end: dictionaries, move
validation, scoring, and persistence all work at that granularity.

<Playground initial={{
  stackOn: true,
  stackScoring: 'sum',
  forbidSame: false,
  useDict: true,
  dictEngine: 'fst',
  rack: ['👩‍🚀', '🛰️', '⭐', '😀', '😀', '😀', '😀']
}} />

## Defining emoji tiles

```json title="Tile configuration"
{
  "tileset": {
    "tile_kinds": [
      {"id": "ASTRONAUT", "symbol": "👩‍🚀", "score": 6},
      {"id": "SATELLITE", "symbol": "🛰️", "score": 5},
      {"id": "STAR", "symbol": "⭐", "score": 4},
      {"id": "BLANK", "symbol": "?", "score": 0, "is_blank": true}
    ]
  },
  "tile_counts": {"ASTRONAUT": 1, "SATELLITE": 1, "STAR": 2, "BLANK": 2}
}
```

Every place the engine surfaces tiles (racks, boards, snapshots) preserves the grapheme string. Stacks
support mixtures of emoji and Latin letters without extra work.

## Dictionaries & normalization

Words are normalized before storage. The default pipeline uses NFC normalization and grapheme
tokenization, which already handles emoji sequences correctly. To customise behaviour (e.g. switch to
NFKC or inject a custom tokenizer), construct dictionaries with `DictionaryOptions` in Rust and then
share the serialized artifact (FST, DAWG, GADDAG) with other bindings.

```rust
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;
use tiletangle_engine::{DictionaryOptions, FstDictionary, Tokenizer, TokenizerRef, NormalizationMode};

struct QuTokenizer;
impl Tokenizer for QuTokenizer {
    fn segment<'a>(&self, text: &'a str) -> Vec<String> {
        text.graphemes(true).collect()
    }
}

let opts = DictionaryOptions {
    case_fold: true,
    norm: NormalizationMode::NFKC,
    tokenizer: TokenizerRef::new(Arc::new(QuTokenizer)),
    min_len: None,
    max_len: None,
};
let dict = FstDictionary::from_words_opts(vec!["👩‍🚀⭐".into(), "שלום".into()], opts);
```

WASM bindings provide helper functions (`set_dictionary_from_text_engine`, `set_dictionary_from_fst_bytes`)
that apply NFC normalization and optional case-folding. For stricter requirements, prebuild an FST/DAWG
with the options above and feed the compiled bytes to the binding API.

## Blanks and marks

When a blank is committed, the engine stores the chosen output string in `Tile.mark`. Rust and WASM
bindings accept an optional `mark` field inside the placement JSON; the value is preserved inside
snapshots and replays. Python, C#, and Godot bindings currently commit blanks with an empty mark; add
marks by replaying the move through the Rust API when you need to persist the chosen glyph.

```ts title="WASM - place a blank as 🛰️"
await play_move(game, JSON.stringify([
  { x: 7, y: 7, kind_id: 'BLANK', mark: '🛰️' },
  { x: 8, y: 7, kind_id: 'ASTRONAUT' }
]));
```

## Regression coverage

- Emoji placements round-trip through snapshots (`snapshot_state_json`) and replays on every binding.
- Tests cover Hebrew (`שלום`) and Arabic (`سلام`) words to ensure grapheme segmentation aligns with
  right-to-left reading direction.
- Custom tokenizer tests confirm DAWG/GADDAG traversal respects multi-codepoint tokens.

## Python example

`examples/python/emoji_demo.py` demonstrates creating an emoji tileset, loading a NFC-normalized
dictionary, and placing moves through the PyO3 binding.
