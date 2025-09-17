---
sidebar_position: 11
---

# Custom Geometry

Phase 7 adds support for graph-based boards: you can define present cells and adjacency explicitly (including hex-like neighbors or boards with holes).

## Config shape

Board layout accepts a `graph` type with `nodes` and `edges`:

```json
{
  "board_layout": {
    "type": "graph",
    "width": 9,
    "height": 9,
    "nodes": [ {"x":0,"y":0}, {"x":1,"y":0}, {"x":0,"y":1} ],
    "edges": [ {"a":0, "b":1, "dir":"E"}, {"a":0, "b":2, "dir":"SE"} ]
  }
}
```

- `nodes` list the coordinates that are part of the board.
- `edges` connect node indices and tag each connection with a direction string (used for line detection and UI hints). Tags like `N, NE, E, SE, S, SW, W, NW` are conventional, but any string works.

If `type` is omitted but `nodes` are present, the engine treats it as a graph layout.

## Rules on graphs

- Line validation: placements must lie along a single direction tag and be contiguous along that line. Gaps are allowed if filled by existing tiles.
- Anchor: moves must touch existing tiles (unless the board is empty). The “center cell” requirement applies only to rectangular boards.
- Scoring: letter bonuses apply to newly placed tiles; word multipliers multiply the entire word (main or cross) that passes through the placed tiles.

## Playground toggle

The docs Playground includes a “Hex adjacency” toggle that constructs a hex-like graph overlay on a rectangular grid. Try placing a short word to see validation and scoring on a non-rectangular board.

## Mapping Graph Coords To 2D

Engine nodes use integer `x, y` coordinates only for identity and relative placement; rendering is up to you. Common mappings:

- Rect grid (orthogonal):
  - Pixel: `px = x * cellSize`, `py = y * cellSize`.
  - Centering/offset: add half‑cell or margins as needed.

- Hex grids (offset coordinates):
  - Even‑R (rows shifted right on even y):
    - `px = (x + 0.5 * (y & 1)) * hexWidth`
    - `py = y * (hexHeight * 0.75)`
  - Even‑Q (columns shifted down on even x):
    - `px = x * (hexWidth * 0.75)`
    - `py = (y + 0.5 * (x & 1)) * hexHeight`
  - Choose a convention and generate edges with matching direction tags (e.g., `E, NE, SE, W, SW, NW`).

- Axial cube coordinates (q, r) → pixel:
  - `px = hexSize * (3/2 * q)`
  - `py = hexSize * (sqrt(3)/2 * (2r + q))`
  - Store `x=q, y=r` in nodes, or convert at load time.

Tips:

- Keep UI coordinate transforms separate from engine nodes; only node indices and tags matter to the validator.
- For boards with holes, simply omit those `nodes`; your renderer can skip them or draw placeholders.
- To render direction hints, reuse edge `dir` tags to draw subtle vectors or to constrain drag‑drop along a tagged line.
