# Tasks — 25-criterion-benchmarks

Meta:
- Spec: 25-criterion-benchmarks — Criterion benchmarks for hot paths + refactor tracking
- Depends on: (none)
- Global scope:
  - Cargo.toml
  - benches/
  - scripts/ (optional)

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T008: Wire `seq.layout` benches to `layout_sequence` (owner: mayor) (scope: benches/seq.rs) (depends: T002)
  - Started_at: 2026-02-08T20:20:43+00:00
  - Completed_at: 2026-02-08T20:22:28+00:00
  - Completion note: Updated `benches/seq.rs` to measure `layout_sequence` (instead of AST checksum) for stable cases (`small`, `medium`, `large_long_text`) with throughput and DCE-safe output consumption.
  - Validation result: `./scripts/bench-criterion run --bench seq -- --sample-size 10` (ok)

- [x] T007: Baseline + comparison workflow docs/scripts (owner: worker:019c3ecf-2f69-70f2-8bc6-b2b24c95f09f) (scope: scripts/) (depends: T001)
  - Started_at: 2026-02-08T19:50:31+00:00
  - Completed_at: 2026-02-08T20:15:27+00:00
  - Completion note: Added `scripts/bench-criterion` helper (run/save/compare/name; baseline name sanitization; defaults `CARGO_HOME` to `/tmp/nereid-cargo-home`) and documented a minimal “save on main, compare on branch” workflow in `scripts/bench-criterion.md`.
  - Validation result: `./scripts/bench-criterion save --bench seq --baseline codex-smoke -- --sample-size 10` and `./scripts/bench-criterion compare --bench seq --baseline codex-smoke -- --sample-size 10` (ok)

- [x] T001: Add Criterion dependency + bench skeletons (owner: worker:019c3e88-187b-74d0-88fb-0f2c45542dc7) (scope: Cargo.toml, Cargo.lock, benches/) (depends: -)
  - Started_at: 2026-02-08T18:33:19+00:00
  - Completed_at: 2026-02-08T18:49:40+00:00
  - Completion note: Added `criterion` as a dev-dependency, configured bench targets with `harness = false`, and created initial `benches/*.rs` skeletons with stable benchmark groups + case IDs to be wired to real workloads in T002+.
  - Validation result: `CARGO_HOME=/tmp/nereid-cargo-home cargo bench` (ok)

- [x] T002: Deterministic fixture generators (owner: worker:019c3e98-a5be-79d1-b3cc-3282b4526e95) (scope: benches/) (depends: T001)
  - Started_at: 2026-02-08T18:51:24+00:00
  - Completed_at: 2026-02-08T19:16:52+00:00
  - Completion note: Added deterministic, parameterized fixture generators under `benches/fixtures/mod.rs` for flowchart DAGs, sequence diagrams, and sessions (including walkthroughs/xrefs), with stable case IDs + checksum helpers; updated bench skeletons to build fixtures outside the timed loop and consume deterministic checksums without changing benchmark IDs.
  - Validation result: `CARGO_HOME=/tmp/nereid-cargo-home cargo bench` (ok)

- [x] T003: Flow layout + routing benches (owner: worker:019c3eb5-676a-77f3-a0e7-609c51c52ad4) (scope: benches/flow.rs) (depends: T002)
  - Started_at: 2026-02-08T19:22:47+00:00
  - Completed_at: 2026-02-08T19:45:58+00:00
  - Completion note: Wired `flow.layout/*` to `layout_flowchart` and `flow.route/*` to `route_flowchart_edges_orthogonal` (layout precomputed), adding stable cases `small`, `medium_dense`, `large_long_labels`, and `routing_stress` with deterministic fixtures, throughput, and DCE-safe output consumption.
  - Validation result: `CARGO_HOME=/tmp/nereid-cargo-home cargo bench --bench flow` (ok)

- [x] T004: Renderer benches (sequence + flowchart) (owner: worker:019c3eb8-2d3b-7d71-8b88-448c60a95317) (scope: benches/render.rs) (depends: T002)
  - Started_at: 2026-02-08T19:25:36+00:00
  - Completed_at: 2026-02-08T19:45:58+00:00
  - Completion note: Implemented renderer benches using canonical layout + Unicode render entrypoints with stable cases: `render.sequence/{small,small_long_text}` and `render.flow/{small,large_long_labels}`; consumes rendered output lengths to avoid DCE.
  - Validation result: `CARGO_HOME=/tmp/nereid-cargo-home cargo bench --bench render` (ok)

- [x] T005: Ops application benches (owner: worker:019c3eb9-ea3a-7e13-85ee-b54352acb5cf) (scope: benches/ops.rs) (depends: T002)
  - Started_at: 2026-02-08T19:27:34+00:00
  - Completed_at: 2026-02-08T19:45:58+00:00
  - Completion note: Added `ops.apply/*` benchmarks over `apply_ops` covering seq+flow op application for single, batch-10, and batch-200 cases; uses `iter_batched` with fresh baseline diagrams, consumes apply outputs, and sets throughput in ops applied.
  - Validation result: `CARGO_HOME=/tmp/nereid-cargo-home cargo bench --bench ops` (ok)

- [x] T006: Store + persistence benches (owner: worker:019c3ecc-f071-7a12-9464-f03bedd5bc1c) (scope: benches/store.rs, benches/scenario.rs) (depends: T002)
  - Started_at: 2026-02-08T19:48:24+00:00
  - Completed_at: 2026-02-08T20:09:51+00:00
  - Completion note: Added `store.save_session` compute-only (Mermaid export + layout + Unicode render, in memory) and I/O benches (`SessionFolder::save_session` into per-iteration temp dirs), plus `scenario.persist_edit` benches that apply a small flow node label update via `apply_ops` then persist with `save_session`, consuming outputs to prevent DCE.
  - Validation result: `CARGO_HOME=/tmp/nereid-cargo-home cargo bench --bench store` and `CARGO_HOME=/tmp/nereid-cargo-home cargo bench --bench scenario` (ok)
