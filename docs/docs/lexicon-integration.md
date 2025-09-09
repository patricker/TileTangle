---
sidebar_position: 6
---

# Lexicon Integration

- Engine uses NFC normalization for symbols and words.
- `SetDictionary` stores words as NFC, with optional case folding.
- Enable dictionary checks by setting `CrosswordRules { free_word_mode: false, .. }` and attaching a dictionary to `GameState`.

Included Word List

- We ship an example list at `assets/dictionaries/TWL06.txt`, sourced from https://scrabutility.com/.
- A prebuilt `.fst` (finite state transducer) variant is generated at `docs/static/dictionaries/TWL06.fst` by `make wasm` for quick loading in the browser.

Example

```rust
let dict = SetDictionary::from_words(vec!["HELLO".into(), "WORLD".into()], true);
state.dictionary = Some(Box::new(dict));
let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
```

Loading From File

```rust
use engine::{SetDictionary, FstDictionary, DictionaryOptions};

let dict = SetDictionary::from_file(
    std::path::Path::new("assets/dictionaries/TWL06.txt"),
    DictionaryOptions { case_fold: true, min_len: Some(2), max_len: None },
).expect("load dictionary");
state.dictionary = Some(Box::new(dict));
let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
```

FST-backed (prefix-capable)

```rust
use engine::{FstDictionary, DictionaryOptions};
let dict = FstDictionary::from_file(
    std::path::Path::new("assets/dictionaries/TWL06.txt"),
    DictionaryOptions { case_fold: true, min_len: Some(2), max_len: None },
).expect("load dictionary");
assert!(dict.has_prefix("ab"));
```

WASM Loader (from bytes)

```js
// Browser: load prebuilt FST and pass bytes to WASM
import init, { new_game, set_dictionary_from_fst_bytes } from '/wasm/engine/pkg/tiletangle_wasm.js';
await init();
const game = new_game(JSON.stringify(cfg), 2);
const resp = await fetch('/dictionaries/TWL06.fst');
const buf = new Uint8Array(await resp.arrayBuffer());
set_dictionary_from_fst_bytes(game, buf, true);
```

Note: richer loaders (metadata, compressed formats) can be added later.
