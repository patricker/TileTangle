# Fuzzing

The harnesses use `libFuzzer` via `cargo fuzz`. Install the tool with `cargo install cargo-fuzz`
(uses nightly) and run, for example:

```bash
cargo fuzz run fuzz_config_load
```

`fuzz_config_load` targets JSON configuration parsing, while `fuzz_move_draft` throws arbitrary move
placements at the crossword validator/commit pipeline. Artifacts land in `fuzz/artifacts/`.
