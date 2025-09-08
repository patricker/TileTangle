# Repository Guidelines

## Project Structure & Module Organization
- Root contains planning docs: `VISION.md` (north star) and `TODO.md` (execution plan).
- `OLD/TileTangle/` holds legacy TypeScript for reference only. Do not run or extend it; it will be deleted.
- New implementation code, when created, should live under a fresh `src/` (and `tests/`).

## Workflow: TODO‑Driven Phases
- Always read `VISION.md` and `TODO.md` before starting any work.
- Work strictly by phases defined in `TODO.md` and fully complete a phase before starting the next.
  - After finishing a phase, update `TODO.md` by marking items complete (e.g., `- [x] Implement parser`).
  - Report a brief summary of work completed and ask to continue to the next phase.
- Keep changes minimal and focused; prefer small, reviewable PRs.

## Build, Test, and Development Commands
- Do not execute anything in `OLD/`.
- Rust workspace commands (root):
  - `make build` or `cargo build --workspace` — compile all crates.
  - `make test` or `cargo test --workspace --all-features` — run tests.
  - `make lint` — Clippy lint with `-D warnings`.
  - `make fmt` — format with rustfmt.

## Coding Style & Naming Conventions
- Default language: TypeScript (unless `TODO.md` specifies otherwise).
- Indentation: 2 spaces; UTF‑8; Unix line endings.
- Names: `PascalCase` (classes/types), `camelCase` (vars/functions), `SCREAMING_SNAKE_CASE` (constants).
- Structure: domain‑oriented folders under `src/`; avoid cyclic imports; keep pure logic separate from I/O.

## Testing Guidelines
- Place tests in `tests/` with `*.test.ts`.
- Use `vitest` or `jest`; target critical logic first. Aim for meaningful coverage, not just line count.
- Provide deterministic fixtures; avoid relying on legacy `OLD/` assets.

## Commit & Pull Request Guidelines
- Conventional Commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.
- PRs must: describe the change and scope, link related TODO items, and include screenshots/logs if UX/dev‑tooling changes.
- Update `TODO.md` in the same PR when completing items.

## Security & Configuration Tips
- Do not commit large/proprietary dictionaries or secrets. Use configs/placeholders and document sources.
- Keep strict compiler/linter settings; justify any relaxations in PR description.
