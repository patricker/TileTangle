---
sidebar_position: 14
---

# Emoji & Graphemes

Tile symbols can be multi-codepoint grapheme clusters (e.g., emoji with skin tones or ZWJ sequences). The engine normalizes inputs (NFC by default) and reads words by grapheme.

## Tips

- Always provide `TileKind.symbol` as the intended grapheme; blanks can map to any runtime symbol via `Tile.mark`.
- For dictionaries, use NFC normalization and case folding as appropriate.

## Python Demo

See `examples/python/emoji_demo.py` for a minimal emoji placement example.

