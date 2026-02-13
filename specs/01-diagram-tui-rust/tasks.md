# Tasks — 01-diagram-tui-rust

Meta:
- Spec: 01-diagram-tui-rust — Rust Diagram TUI (AST + MCP + Walkthroughs)
- Depends on: -
- Global scope:
  - Cargo.toml
  - src/
  - tests/
  - docs/
  - specs/index.md
  - specs/_handoff.md
  - specs/01-diagram-tui-rust/

## In Progress

## Blocked

## Todo

- (none; work has been split into subsystem specs — see `specs/index.md`)

## Done

- [x] T900: Spec split pass — split umbrella spec into subsystem specs (owner: mayor) (scope: specs/) (depends: -)
  - Completed_at: 2026-02-07T01:18:54+00:00
  - Completion note: Split the original “all-in-one” plan into subsystem specs so multiple workers can work in parallel on disjoint scopes (model/ops/format/render/layout/query/store/tui/mcp).
  - Validation result: N/A (planning change)

- [x] T002: Implement core IDs and `ObjectRef` parsing/formatting (owner: worker:019c3584-565f-7861-b368-4d50400bdfe0) (scope: src/model/) (depends: T001)
  - Started_at: 2026-02-07T00:32:33+00:00
  - DoD: stable ID types exist; `ObjectRef` round-trips; unit tests for parsing and canonical formatting.
  - Validation: `cargo test`
  - Escalate if: `ObjectRef` encoding cannot represent required categories cleanly.
  - Completed_at: 2026-02-07T00:42:36+00:00
  - Completion note: Implemented std-only typed IDs and canonical `ObjectRef` parsing/formatting (`d:<diagram_id>/<category...>/<object_id>`), including error handling and unit tests covering protocol examples + failure cases.
  - Validation result: `cargo test` (ok)

- [x] T001: Initialize Rust project skeleton (owner: worker:019c357e-7616-7793-b019-2f54e8f0f844) (scope: Cargo.toml,src/) (depends: -)
  - Started_at: 2026-02-07T00:26:06+00:00
  - DoD: `cargo test` runs; baseline module layout created per `design.md`.
  - Validation: `cargo test`
  - Escalate if: dependency choices for `ratatui`/MCP are unclear; stop before locking crates.
  - Completed_at: 2026-02-07T00:30:44+00:00
  - Completion note: Initialized a std-only Rust single-crate skeleton with baseline module layout (`model/ops/query/format/layout/render/store/tui/mcp`) per `design.md` and a minimal sanity test.
  - Validation result: `cargo test` (ok)
