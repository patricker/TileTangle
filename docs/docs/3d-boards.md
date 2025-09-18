---
sidebar_position: 12
---

# 3D Boards

TileTangle supports multi-layer boards by flattening each slice into the base grid and connecting neighbours along the Z-axis with a graph overlay. Validation and scoring treat those tagged edges the same way as X/Y neighbours, so words can bend between layers.

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

- Toggle “3D (layers)” in the Playground controls and set the desired `Depth`.
- Use the `Slice z` slider to browse layers and drag tiles within the active slice. Placements can span layers as long as they stay contiguous along X/Y/Z.

## Notes

- 3D is implemented via the general graph layer; future work can add true 3D coordinate helpers.
- Demos can render slice stacks (e.g., Godot 3D) using the same JSON API.
