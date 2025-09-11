---
sidebar_position: 12
---

# 3D Boards

Phase 8 introduces 3D boards by layering 2D slices and connecting neighbors along the Z‑axis. The engine models this via a graph overlay, tagging edges with direction families (X, Y, Z). Validation and scoring operate along these tags.

## Config: 3×3×3 Toy Board

```json
{
  "board_layout": {
    "type": "3d",
    "width": 3,
    "height": 3,
    "depth": 3
  }
}
```

- The engine derives a flattened grid internally. Adjacency is defined across X, Y, and Z, enabling straight lines across layers.
- Bonus cells apply normally; word multipliers multiply the entire word formed along a line.

## Playground

- Toggle “3D (layers)” in the Playground and choose `Depth`. Use the `Slice z` slider to browse layers.
- Drag tiles to place within the active slice; lines can span across Z if contiguous.

## Notes

- 3D is implemented via the general graph layer; future work can add true 3D coordinate helpers.
- Demos can render slice stacks (e.g., Godot 3D) using the same JSON API.

