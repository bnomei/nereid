# Tasks — 15-audit

Meta:
- Spec: 15-audit — Implementation audit report
- Depends on: (none)
- Global scope:
  - docs/audit.md

## In Progress

## Blocked

## Todo

## Done

- [x] T005: Compile `docs/audit.md` (owner: mayor) (scope: docs/audit.md) (depends: T001–T004)
  - Started_at: 2026-02-07T13:25:09+00:00
  - DoD: `docs/audit.md` created with consolidated findings + severity + proposed follow-ups.
  - Validation: `cargo test --offline`; `cargo clippy --offline` (warnings recorded).
  - Completed_at: 2026-02-07T13:27:04+00:00
  - Completion note: Compiled worker audit findings plus additional checks into `docs/audit.md`, including prioritized P1/P2 issues and suggested next-wave tasks. Recorded current validation status (`cargo test --offline` ok; `cargo clippy --offline` ok with 29 warnings).
- [x] T001: Audit persistence + store safety (owner: worker:019c383d-9e12-7323-9290-aa3a2aeac718) (scope: src/store/) (depends: none)
  - Started_at: 2026-02-07T13:13:29+00:00
  - DoD: written findings covering session folder load/save, relative path safety, rev/xref persistence, and any missing wiring.
  - Validation: none (read-only)
  - Completed_at: 2026-02-07T13:23:15+00:00
  - Completion note: Audited `SessionFolder` persistence and IO safety: relative-path JSON fields are validated (no `..`/absolute), but symlink redirection can escape the session root for writes; also `Session.xrefs`, `Session.active_walkthrough_id`, and `Diagram.rev` are not round-tripped by `save_session/load_session`, and walkthrough directory scanning can resurrect removed walkthroughs. Proposed follow-up tasks cover symlink/atomic-write hardening and completing persistence wiring with round-trip tests.

- [x] T002: Audit MCP + ops/delta semantics (owner: worker:019c383d-a2d5-7381-a719-6b56f7fb2fb6) (scope: src/mcp/, src/ops/) (depends: none)
  - Started_at: 2026-02-07T13:13:29+00:00
  - DoD: written findings covering tool schemas, conflict handling, delta behavior, and any “not wired” gaps (e.g., persistence hooks).
  - Validation: none (read-only)
  - Completed_at: 2026-02-07T13:21:47+00:00
  - Completion note: Audited MCP tool surface (param validation, error mapping, determinism, concurrency), reviewed delta semantics (base_rev gating, last-delta cache), and audited ops patch application; found P1 gaps in sequence message endpoint validation and non-recoverable delta sync without snapshot/history, plus several P2/P3 semantics/ergonomics issues; proposed narrowly-scoped follow-ups with tests.

- [x] T003: Audit rendering/layout/TUI correctness + perf (owner: worker:019c383d-a6df-7772-b3c8-4db3b1141959) (scope: src/render/, src/layout/, src/tui/) (depends: none)
  - Started_at: 2026-02-07T13:13:29+00:00
  - DoD: written findings covering determinism, caching, potential panics, and any perf hotspots.
  - Validation: none (read-only)
  - Completed_at: 2026-02-07T13:21:14+00:00
  - Completion note: Audited rendering/layout/TUI for determinism, panic paths, and perf hotspots. Layout/rendering is generally deterministic (BTreeMap/BTreeSet + explicit sorting) with snapshot/unit tests, but flowchart routing uses panic-based failure paths and per-edge BFS that can hurt robustness/perf at scale. The TUI shell works for static sessions but lacks diagram switching/refresh, limiting session-backed rendering and cross-diagram XRef navigation; follow-ups cover removing panics, improving routing/UI performance, and addressing Unicode width handling.

- [x] T004: Audit parsing/export + model/query robustness (owner: mayor) (scope: src/format/, src/model/, src/query/) (depends: none)
  - Started_at: 2026-02-07T13:13:29+00:00
  - DoD: written findings covering input handling, round-tripping expectations, invariants, and edge cases that could lead to panics/misalignment.
  - Validation: none (read-only)
  - Completed_at: 2026-02-07T13:25:09+00:00
  - Completion note: Audited Mermaid parsing/export plus model/query robustness. Found that sequence export currently does not validate/escape message text (can emit invalid `.mmd` if AST is mutated via ops), the model contains sequence notes/roles not yet wired into parse/export/render, IDs are permissive (only disallow `/`) which can cause filesystem portability issues for persisted diagram ids, and session route derivation recomputes adjacency per query (fine for v1 but can be a hotspot at scale).
