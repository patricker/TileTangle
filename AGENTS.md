# Repository Guidelines

This file sets expectations for contributors and automation working in this repository. It applies repo‑wide.

## Docs Style Guide

Audience and purpose

- Write for human readers (players, integrators, developers), not for agents.
- Keep docs self‑contained, practical, and task oriented. Favor examples that work as shown.

Voice and tense

- Use clear, direct, present‑tense language. Describe what the engine does now.
- Avoid speculative phrasing: do not use “will”, “could add”, “future”, “Phase X”, or similar roadmap talk.
- Prefer “the engine” / “this page” over “we/I”.

Internal references

- Do not mention internal planning files or processes in user docs (e.g., no references to TODO.md, VISION.md phases, “Appendix A”, or roadmap items).
- If background is necessary, link to public docs pages, not internal notes.

Trademarks and proper nouns

- Do not reference trademarked or copyrighted games by name.
- Use neutral descriptors instead, for example:
  - “classic 15×15 crossword‑style board”
  - “anchor‑based move generation”, “cross‑check sets”
  - “word bonus”, “letter multiplier”, “bingo bonus”

Benchmarks and numbers

- Treat benchmark numbers as a snapshot. Do not instruct readers to change files or “update the chart”.
- Provide a “Reproduce locally” command and explain the workload, batch sizes, and variability across hardware/toolchains.
- Qualify environment details (CPU, compiler) only when they aid interpretation.

CI and examples

- Describe CI/perf gates as examples unless a public workflow exists. Avoid phrasing that implies guarantees.
- Prefer “Example gate: fail on ≥2× slowdown vs. baseline” over “CI runs X and fails the build”.

Examples must run as written

- Keep examples self‑contained. If a page shows emoji or custom tiles, define those tile kinds and counts inline.
- Ensure dictionary settings match the example content (or explicitly disable dictionary checks for non‑lexical demos).
- Avoid hidden dependencies on environment defaults.

Terminology and consistency

- Use consistent terms across pages (anchor, cross‑check, rack, prefix search, etc.). Define concepts on first use.
- Prefer engine‑agnostic terms over brand‑specific ones.

Tone and scope

- Do not include contributor‑ or bot‑oriented instructions in user docs.
- Keep claims factual; avoid marketing language and future promises.

Change management

- Keep edits minimal and focused. Prefer small, reviewable diffs.
- When updating examples, verify they run with the current repository state.

