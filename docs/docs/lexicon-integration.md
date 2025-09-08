---
sidebar_position: 6
---

# Lexicon Integration

- Engine uses NFC normalization for symbols and words.
- `SetDictionary` stores words as NFC, with optional case folding.
- Enable dictionary checks by setting `CrosswordRules { free_word_mode: false, .. }` and attaching a dictionary to `GameState`.

Included Word List

- We ship an example list at `assets/dictionaries/TWL06.txt`, sourced from https://scrabutility.com/.

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

Note: richer loaders (metadata, compressed formats) can be added later.
