# Tasks — 02-dependencies

Meta:
- Spec: 02-dependencies — Cargo dependencies + feature flags
- Depends on: -
- Global scope:
  - Cargo.toml
  - Cargo.lock

## In Progress

## Blocked

## Todo

- (none)

## Done

- [x] T008: Add `pprof` as a dev-dependency for flamegraphs (owner: mayor) (scope: Cargo.toml,Cargo.lock) (depends: -)
  - DoD: `pprof = { version = "0.15.0", features = ["flamegraph"] }` added under `[dev-dependencies]`; `cargo test` remains green.
  - Completed_at: 2026-02-09T13:18:23+00:00
  - Completion note: Added `pprof` as a dev-dependency so benchmarks can optionally produce flamegraphs using existing Criterion tooling.
  - Validation result: `cargo test` (ok)

- [x] T007: Add `rayon` as an always-available dependency (owner: mayor) (scope: Cargo.toml,Cargo.lock) (depends: -)
  - DoD: `rayon = "1"` added as a direct dependency; `cargo test` remains green.
  - Completed_at: 2026-02-09T13:18:23+00:00
  - Completion note: Added `rayon` as a non-optional dependency so perf refactors can use parallelism where benchmarked wins justify overhead. Keep usage selective in code (thresholds/bench-gated).
  - Validation result: `cargo test` (ok)

- [x] T006: Add micro-opt crates for perf refactors (owner: mayor) (scope: Cargo.toml,Cargo.lock) (depends: -)
  - DoD: Added direct dependencies `itoa`, `memchr`, `smallvec`, `smol_str`; `cargo test` remains green.
  - Completed_at: 2026-02-09T13:18:23+00:00
  - Completion note: Added small, targeted utility crates so upcoming hot-path refactors can use them without manifest churn.
  - Validation result: `cargo test` (ok)

- [x] T005: Add direct deps needed for rmcp server runtime + schemas (`tokio`, `schemars`) (owner: mayor) (scope: Cargo.toml) (depends: T004)
  - DoD: direct deps added; build/tests remain green offline.
  - Completed_at: 2026-02-07T12:32:15+00:00
  - Completion note: Added `tokio` (for rmcp server runtime + stdio handles) and `schemars` (so `JsonSchema` derives work in this crate) as direct dependencies; no new lock entries beyond what rmcp already pulled in, validated with offline tests.
  - Validation result: `cargo test --offline` (ok)

- [x] T004: Add `rmcp` v0.14.0 for MCP server implementation (owner: worker:019c37f8-9e83-7091-a5b1-dc77845a0215) (scope: Cargo.toml,Cargo.lock) (depends: -)
  - Started_at: 2026-02-07T11:59:33+00:00
  - DoD: crate builds; `cargo test` remains green; `Cargo.toml`/`Cargo.lock` updated.
  - Completed_at: 2026-02-07T12:07:13+00:00
  - Completion note: Added `rmcp = "0.14.0"` and regenerated `Cargo.lock` successfully using offline mode (`cargo generate-lockfile --offline`) to avoid network/index resolution. Confirmed the workspace builds and tests pass offline.
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Decide MCP stack under offline constraints (owner: mayor) (scope: Cargo.toml,Cargo.lock) (depends: -)
  - Context (worker-facing):
    - Network access is restricted in this environment; `cargo search` cannot reach crates.io.
    - Prefer **no new dependencies** for MCP v1. Use stdio + existing `serde_json` for a temporary dev harness (see `12-mcp/T001` context).
  - DoD: `Cargo.toml` remains unchanged (unless a dependency is already available locally and does not require network access); `12-mcp/T001` can proceed without waiting on any new crates.
  - Validation: `cargo test`
  - Escalate if: a new crate is truly required and cannot be fetched offline; propose a vendoring/offline plan and stop.
  - Completed_at: 2026-02-07T11:42:20+00:00
  - Completion note: Chose a stdio JSON-line harness for MCP v1 to avoid new dependencies under offline constraints; later superseded by user decision to use `rmcp` v0.14.0 (see `T004`).
  - Validation result: `cargo test` (ok)

- [x] T002: Add TUI stack (`ratatui` + backend) (owner: worker:019c37a1-c209-7213-907e-b1066f2e3f06) (scope: Cargo.toml) (depends: -)
  - Started_at: 2026-02-07T10:24:25+00:00
  - DoD: `ratatui` and chosen backend added; crate still builds/tests.
  - Validation: `cargo test`
  - Escalate if: dependency choice is unclear; do not guess without a short decision note in `design.md`.
  - Completed_at: 2026-02-07T10:29:19+00:00
  - Completion note: Added `ratatui` (crossterm backend) and `crossterm` dependencies with minimal features; updated `Cargo.lock`; `cargo test` is green.
  - Validation result: `cargo test` (ok)

- [x] T001: Add serialization stack for store (owner: worker:019c35b5-9c12-7f32-b92d-ccaf3bc2c892) (scope: Cargo.toml) (depends: -)
  - Started_at: 2026-02-07T01:25:47+00:00
  - DoD: `serde` + JSON backend (or alternative) selected and added; `cargo test` remains green.
  - Validation: `cargo test`
  - Escalate if: network restrictions prevent fetching crates; stop and propose an offline strategy.
  - Completed_at: 2026-02-07T01:35:45+00:00
  - Completion note: Added `serde` (derive) and `serde_json` to `Cargo.toml` for session/store serialization; updated `Cargo.lock`.
  - Validation result: `cargo test` (ok)
