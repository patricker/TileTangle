---
sidebar_position: 6
---

# Lexicon Integration

- Engine uses NFC normalization for symbols and words.
- `SetDictionary` stores words as NFC, with optional case folding.
- Enable dictionary checks by setting `CrosswordRules { free_word_mode: false, .. }` and attaching a dictionary to `GameState`.

Included Word List

- An example list is included at `assets/dictionaries/TWL06.txt`, sourced from https://scrabutility.com/.
- A prebuilt `.fst` (finite state transducer) variant is generated at `docs/static/dictionaries/TWL06.fst` by `make wasm` for quick loading in the browser.

Example

```rust
let dict = SetDictionary::from_words(vec!["HELLO".into(), "WORLD".into()], true);
state.dictionary = Some(Box::new(dict));
let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
```

Loading From File

```rust
use engine::{SetDictionary, FstDictionary, DictionaryOptions, TokenizerRef};

let dict = SetDictionary::from_file(
    std::path::Path::new("assets/dictionaries/TWL06.txt"),
    DictionaryOptions { case_fold: true, min_len: Some(2), max_len: None, tokenizer: TokenizerRef::default() },
).expect("load dictionary");
state.dictionary = Some(Box::new(dict));
let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
```

FST-backed (prefix-capable)

```rust
use engine::{FstDictionary, DictionaryOptions, TokenizerRef};
let dict = FstDictionary::from_file(
    std::path::Path::new("assets/dictionaries/TWL06.txt"),
    DictionaryOptions { case_fold: true, min_len: Some(2), max_len: None, tokenizer: TokenizerRef::default() },
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

WASM Loader (choose engine)

```js
import init, { new_game, set_dictionary_from_text_engine } from '/wasm/engine/pkg/tiletangle_wasm.js';
await init();
const game = new_game(JSON.stringify(cfg), 2);
const txt = await (await fetch('/dictionaries/TWL06.txt')).text();
// engine can be 'set', 'fst', 'dawg', or 'gaddag'
set_dictionary_from_text_engine(game, txt, 'gaddag', true);
```

Note: the engine supports loading from text and prebuilt FST blobs. If you need other formats or metadata
packaging, load your words at runtime and construct the desired dictionary via the public API.

## Licensing & Packaging Guidance

- Prefer tiny, permissively licensed test lists in-repo; avoid committing large or proprietary word lists.
- Document sources and licenses for any example lists; verify redistribution terms before bundling.
- Provide configuration to load real lexica at runtime (paths, URLs, or user-uploaded files), rather than vendoring them.

Recommended approaches by target:

- Desktop/Server (Rust, Python):
  - Load from a local path via `SetDictionary::from_file` or `FstDictionary::from_file`.
  - Keep real dictionaries outside the repository; point to them via a config file or env var (e.g., `WORDENGINE_DICT_PATH`).
  - For performance, prefer FST-backed dictionaries once built.

- Web (WASM):
  - Prebuild an FST from a source list locally and host the resulting `.fst` yourself. In docs/site deployments, place it under `docs/static/dictionaries/` or serve from your own CDN.
  - Alternatively, allow users to upload a list at runtime and convert client-side or on a backend, then feed bytes to the WASM API (see the FST example above).

- Python:
  - Expose a path-based loader and keep packaging wheel sizes small by not bundling large lists.
  - Ship only minimal fixtures for tests and examples.

Notes on common lists:

- Tournament lists like TWL/OWL/CSW often have licensing restrictions. Treat any included copy as demo-only and replace with your own sources when distributing your app.
- When in doubt, do not redistribute: instruct users to supply their own dictionary files, and provide tooling/docs to consume them.

Build tips:

- Create a small public-domain or self-authored test list (dozens/hundreds of words) for CI and examples.
- Provide a helper script/Make target to transform a plain `.txt` list into an `.fst` for production use; keep the large source file out of version control.
