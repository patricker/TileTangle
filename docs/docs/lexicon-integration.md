---
sidebar_position: 6
---

# Lexicon Integration

- Engine uses NFC normalization for symbols and words.
- `SetDictionary` stores words as NFC, with optional case folding.
- Enable dictionary checks by setting `CrosswordRules { free_word_mode: false, .. }` and attaching a dictionary to `GameState`.

Example

```rust
let dict = SetDictionary::from_words(vec!["HELLO".into(), "WORLD".into()], true);
state.dictionary = Some(dict);
let rules = CrosswordRules { free_word_mode: false, ..Default::default() };
```

Note: richer loaders (files, language metadata) arrive in later phases.
