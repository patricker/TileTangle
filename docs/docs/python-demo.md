---
title: Python Demo
---

Run the minimal CLI example to play a move and print the score and board JSON.

Steps

- `pip install maturin`
- `cd bindings/python && maturin develop --release`
- `python examples/python/mini_cli.py`

Expected output (example):

```
Board: {"width":5,"height":5,"rows":[["",...]]}
Score: {'total': 4, 'main_word': 'AB', 'main_score': 4, 'cross_words': [], 'bingo': False}
```

