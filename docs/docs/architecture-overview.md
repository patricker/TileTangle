---
sidebar_position: 2
---

# Architecture Overview

TileTangle targets a Rust core engine with bindings (WASM/JS, Python, Unity, Godot).

- Core crate: deterministic, no platform deps; feature flags gate integrations.
- Bindings expose a JSON-friendly API for config/state and moves.
- Docs site hosts live WASM demos and language examples.

This page summarizes core components. See linked pages for deeper dives.
