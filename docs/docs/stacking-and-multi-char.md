---
id: stacking-and-multi-char
title: Stacking & Multi-Char Tiles
description: Overlay rules, scoring variants, and Unicode-friendly tile symbols.
---

Stacking (Upwords-like)

- Enable overlays with `CrosswordRules.stacking_enabled = true`.
- Constraints:
  - `stacking_max_height`: limit total tiles per cell.
  - `forbid_same_symbol_overlay`: prevent overlaying the same visible symbol.
- Scoring modes:
  - `TopOnly`: only top tile contributes; letter multipliers apply to newly placed tops only.
  - `SumStack`: sum underlying tiles plus top (with multipliers); word multipliers apply as usual.

Multi-Character / Emoji Tiles

- `TileKind.symbol` can be any Unicode string (grapheme sequences and ZWJ emoji supported).
- Dictionary engines operate on normalized strings (NFC by default, optional NFKC).
- Move generation treats multi-character symbols as a single tile; cross-checks count tiles, not codepoints.

WASM Playground

- Try the new toggles:
  - “Stacking”: overlay tiles; switch scoring between TopOnly and SumStack.
  - “RTL reading”: reverse horizontal word assembly for dictionary checks.

