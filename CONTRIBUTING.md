# Contributing

Thank you for your interest in TileTangle.

- Read `VISION.md` to understand the goals and constraints.
- See `AGENTS.md` (Repository Guidelines) for coding style, testing, and PR requirements.

For questions or proposals, open a focused discussion or PR with a clear scope.

## Local Checks

- Run `make test` for the default developer loop. It now lints engine test code with clippy (`-D warnings`) before executing the full workspace test suite, helping you catch the same issues CI enforces.
- Run `make lint-core` to run strict clippy on the core crates only (engine and ffi), mirroring CI’s current focus.
- Run `make check` to run format, full-clippy (all targets) and tests in one go.
- Install pre-commit hooks (`pip install pre-commit && pre-commit install`) to automatically run `rustfmt` and workspace clippy on commit.
