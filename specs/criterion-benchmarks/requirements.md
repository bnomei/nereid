# Requirements — 25-criterion-benchmarks

Context
- Goal: add a Criterion benchmark suite that can **track performance across refactors** (not just “one-off profiling”).
- The codebase has known hot paths in routing/layout/render, ops apply, and persistence (store + MCP persistent mode).
- Some duplicated render helpers (e.g. trimming/ellipsis/text length) are expected to be deduplicated soon; the benchmark plan must remain valid and continue measuring the **canonical** implementations after those refactors land.

Goals
- Provide stable, repeatable benchmarks for the identified hot paths.
- Make benchmark case names/IDs stable so results remain comparable across refactors.
- Ensure benchmarks are resilient to internal code movement/dedup (update mappings, not benchmark identity).

Non-goals
- CI performance gating (hard fail on regressions) in this wave.
- Perf counters or platform-specific tooling (pprof flamegraphs are optional; see `scripts/bench-criterion.md`).
- Microbenching private helper functions whose location is expected to churn; prefer stable entrypoints.

## Requirements (EARS)

- The system shall provide a Criterion benchmark suite runnable via `cargo bench`.
- The system shall define a stable benchmark naming scheme (group + case id) that does not change across internal refactors.
- When internal implementations are moved or deduplicated, the system shall update benchmark wiring so the same benchmark IDs continue to measure the updated canonical implementations.
- The system shall generate deterministic fixtures (no nondeterministic randomness) for all benchmarks.
- The system shall prevent dead-code elimination by consuming benchmark outputs (e.g. via a checksum or `black_box`).
- The system shall include benchmark coverage for these workloads:
  - flowchart layout and orthogonal routing
  - sequence layout
  - diagram rendering (sequence + flowchart)
  - ops application (batch sizes and mixes)
  - session persistence/export (`SessionFolder::save_session`) in both compute-only and filesystem modes
  - “persistent edit” scenario: apply a small change then persist (to track future incremental-save refactors)
- While running filesystem benchmarks, the system shall isolate benchmark output to a per-run temporary directory to avoid cross-run contamination.

## Validation

- `cargo bench`
- Optional baseline workflow (Criterion):
  - save baseline on `main`
  - re-run on refactor branch using the same benchmark IDs for comparison
