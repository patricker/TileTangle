# Benchmarks

Criterion benchmarks live under `engine/benches/`. Run them with:

```bash
cargo bench -p tiletangle-engine
```

The suite exercises dictionary lookups, move generation, and the AI evaluator. HTML reports land in
`target/criterion`. Enable the optional parallel evaluator with:

```bash
cargo bench -p tiletangle-engine --features parallel
```
