---
title: Language Packs & Tokenization
description: Configure normalization, reading direction, and custom tokenization for multilingual word sets.
---

TileTangle treats language behaviour as data. The same engine can validate English crosswords,
Hebrew crosswords, emoji anagrams, or half-width Japanese tiles—no code changes required.

> **Tip:** Build dictionaries with the Rust API when you need advanced normalization or custom
> tokenizers, then reuse the compiled artifacts (FST, DAWG, GADDAG) from Python, WASM, Unity, or Godot.

## Normalization & case-folding

`DictionaryOptions` controls how input words are normalized before being stored. NFC is the default,
but you can opt into NFKC (useful for ligatures and half-width kana) or disable case folding entirely.

```rust
use tiletangle_engine::{DictionaryOptions, FstDictionary, NormalizationMode, TokenizerRef};

let opts = DictionaryOptions {
    norm: NormalizationMode::NFKC,
    case_fold: false,
    tokenizer: TokenizerRef::default(),
    min_len: None,
    max_len: None,
};
let dict = FstDictionary::from_words_opts(words, opts);
```

With this configuration a tile placed as `"ﾊﾟ"` will match the dictionary entry `"パ"`.

## Reading direction

Set `CrosswordRules::reading_dir` to `ReadingDirection::RTL` for scripts like Hebrew or Arabic. The
validation and scoring pipeline now reads horizontal words right-to-left while continuing to render
on a standard grid. WASM/Unity/Godot expose `set_reading_direction(rtl)` for the same behaviour.

```rust
let mut rules = CrosswordRules { free_word_mode: false, ..Default::default() };
rules.reading_dir = ReadingDirection::RTL;
```

Our regression tests cover Hebrew (`שלום`) and Arabic (`سلام`) placements to ensure glyph ordering
remains correct.

## Tokenization hooks

Not every language splits neatly into Unicode scalars. Emoji sequences, digraph tiles (`QU`), or
stacked accents often need custom segmentation. Implement the `Tokenizer` trait and hand it to the
dictionary via `TokenizerRef`:

```rust
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;
use tiletangle_engine::{Tokenizer, TokenizerRef, DictionaryOptions, DawgDictionary};

struct QuTokenizer;
impl Tokenizer for QuTokenizer {
    fn segment<'a>(&self, text: &'a str) -> Vec<String> {
        let mut out = Vec::new();
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == 'q' && chars.peek() == Some(&'u') {
                chars.next();
                out.push("qu".into());
            } else {
                out.push(ch.to_string());
            }
        }
        out
    }
}

let tokenizer = TokenizerRef::new(Arc::new(QuTokenizer));
let opts = DictionaryOptions {
    tokenizer,
    ..DictionaryOptions::default()
};
let dawgd = DawgDictionary::from_words_opts(vec!["squid".into()], opts);
assert!(dawgd.has_prefix("squ"));

WASM bindings automatically use the default grapheme tokenizer. To reuse a custom tokenizer there,
construct the dictionary in Rust (as above) and load the serialized bytes via
`set_dictionary_from_fst_bytes`.

The same tokenizer automatically flows into `GaddagDictionary` and the move generator, ensuring
prefix walks and cross-checks stay in sync.

## Multi-grapheme & stacked tiles

Tiles may already carry multi-codepoint symbols (`"👨‍👩‍👧‍👦"`, hangul syllables, etc.). Because
words are now reconstructed through the tokenizer, each stack entry is treated as one logical
token—regardless of how many scalars it contains.
