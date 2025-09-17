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

## Baseline comparisons

CI keeps an eye on regressions by diffing every run against `bench/baselines/main.json`. After you
run the Criterion suite locally, print the same summary with:

```bash
./tools/report_bench.py --baseline bench/baselines/main.json
```

If you intentionally improve performance and want to refresh the baseline, regenerate it from the
latest results:

```bash
./tools/report_bench.py --baseline bench/baselines/main.json --write-baseline
```

The script exits non-zero when a benchmark regresses by 2× or more, mirroring the CI gate.
