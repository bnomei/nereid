# Tasks — 16-audit-remediation

Meta:
- Spec: 16-audit-remediation — Audit follow-ups + Clippy zero-warnings
- Source list: `docs/audit.md`
- Depends on: spec:15-audit/T005
- Global scope:
  - docs/audit.md
  - src/store/
  - src/model/
  - src/layout/
  - src/render/
  - src/format/
  - src/query/
  - src/ops/
  - src/tui/
  - src/mcp/
## In Progress

## Blocked

## Todo

## Done

- [x] T005: Optional session-route adjacency reuse (avoid recompute hotspots) (owner: worker:019c3a56-e721-7563-9cd8-72873b5e09c1) (scope: src/query/) (depends: spec:08-query-engine/T003)
  - Completed_at: 2026-02-08T00:48:51+00:00
  - Completion_note: Added SessionRouteAdjacency + find_route_with_adjacency(...) for adjacency reuse; kept find_route unchanged; added parity tests (cargo test --offline: ok, 189 passed).
  - Started_at: 2026-02-07T22:57:47+00:00
  - Context (from `docs/audit.md`): session route derivation recomputes adjacency per query.
  - DoD:
    - Provide an optional pre-derived adjacency representation and a `find_route_with_adjacency(...)` API so callers can reuse adjacency for repeated queries.
    - Keep the existing `find_route(session, from, to)` API unchanged.
    - Add unit tests proving the cached and uncached route APIs return identical results.
  - Validation: `cargo test --offline`
  - Escalate if: this requires adding a session-level rev/invalidation mechanism; keep this task API-only.

- [x] T006: Flowchart routing scaling pass (allocation + data structure hot spots) (owner: worker:019c3a57-6042-7862-86b1-c52d1054669a) (scope: src/layout/flowchart.rs) (depends: spec:07-layout-engine/T001)
  - Completed_at: 2026-02-08T00:48:51+00:00
  - Completion_note: Reused scratch buffers in flowchart routing to avoid per-route/per-step allocations; added determinism test (cargo test --offline: ok, 189 passed).
  - Started_at: 2026-02-07T22:57:47+00:00
  - Context (from `docs/audit.md`): flowchart routing performs BFS per edge using `BTreeMap/BTreeSet` and can grow expensive.
  - DoD:
    - Reduce avoidable per-route allocations (reuse scratch buffers where feasible).
    - Keep routing deterministic and preserve current routing behavior for existing tests.
    - Add at least one new test case that stresses routing with a moderately-sized fixture and asserts it remains deterministic.
  - Validation: `cargo test --offline`
  - Escalate if: this turns into an algorithm rewrite; keep this task to local optimizations only.

- [x] T004: Encode persisted ids into OS-portable filenames (owner: unassigned) (scope: src/store/) (depends: T002,T003,spec:09-session-store/T006)
  - Completed_at: 2026-02-08T00:48:51+00:00
  - Completion_note: Encoded persisted IDs into filename-safe segments (Windows-safe) with legacy fallback; tests cover unsafe IDs (cargo test --offline: ok, 189 passed; cargo clippy --offline: ok).
  - Context (from `docs/audit.md`): IDs are permissive but used in file names (portability risk on Windows).
  - DoD:
    - Introduce a store-local encoding for file-name segments (keep model IDs unchanged).
    - Use encoded ids when constructing default on-disk paths for diagrams/walkthroughs.
	    - Backwards compatibility:
	      - Existing sessions that persisted legacy filenames remain loadable (paths in `nereid-session.meta.json` remain authoritative).
      - For id-derived paths (e.g. `load_walkthrough(id)`), support both encoded and legacy filenames.
    - Unit tests cover encoded ids containing characters unsafe on Windows (e.g. `:`) and verify save/load correctness.
  - Validation: `cargo test --offline`

- [x] T001: Refresh audit doc status + close resolved findings (owner: mayor) (scope: docs/audit.md) (depends: -)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Context: `docs/audit.md` status + findings are stale compared to the current codebase.
  - DoD:
    - Update `docs/audit.md` to reflect current `cargo test --offline` + `cargo clippy --offline` status.
    - Mark findings that are now resolved as closed (with brief references to the regression tests or modules).
    - Leave remaining open findings as actionable follow-ups pointing to this spec’s tasks.
  - Validation: `cargo test --offline`; `cargo clippy --offline`

- [x] T010: Clippy cleanup — flowchart parser (`manual_strip`) (owner: mayor) (scope: src/format/mermaid/flowchart.rs) (depends: -)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Started_at: 2026-02-07T21:38:50+00:00
  - DoD: `cargo clippy --offline` emits no warnings originating from `src/format/mermaid/flowchart.rs`.
  - Validation: `cargo clippy --offline`; `cargo test --offline`

- [x] T011: Clippy cleanup — MCP server (to_vec / redundant_closure / unnecessary_to_owned) (owner: mayor) (scope: src/mcp/server.rs) (depends: -)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Started_at: 2026-02-07T21:38:50+00:00
  - DoD: `cargo clippy --offline` emits no warnings originating from `src/mcp/server.rs`.
  - Validation: `cargo clippy --offline`; `cargo test --offline`

- [x] T012: Clippy cleanup — ops delta helpers (`ptr_arg`) (owner: mayor) (scope: src/ops/mod.rs) (depends: -)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Started_at: 2026-02-07T22:08:29+00:00
  - DoD: `cargo clippy --offline` emits no warnings originating from `src/ops/mod.rs`.
  - Validation: `cargo clippy --offline`; `cargo test --offline`

- [x] T013: Clippy cleanup — flow query Tarjan helper (`too_many_arguments`) (owner: mayor) (scope: src/query/flow.rs) (depends: -)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Started_at: 2026-02-07T22:17:40+00:00
  - DoD: `cargo clippy --offline` emits no warnings originating from `src/query/flow.rs`.
  - Validation: `cargo clippy --offline`; `cargo test --offline`

- [x] T014: Clippy cleanup — render utilities (`manual_is_multiple_of`, range pattern) (owner: mayor) (scope: src/render/) (depends: -)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Started_at: 2026-02-07T22:08:29+00:00
  - DoD: `cargo clippy --offline` emits no warnings originating from `src/render/`.
  - Validation: `cargo clippy --offline`; `cargo test --offline`

- [x] T015: Clippy cleanup — store (`result_large_err`, `io_other_error`) (owner: mayor) (scope: src/store/session_folder.rs) (depends: T004)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Context: Clippy reports many `result_large_err` warnings for store helpers, plus a couple of `io_other_error` warnings.
  - DoD:
    - Eliminate `result_large_err` warnings in store (prefer structural fixes over blanket `#[allow]`).
    - Apply the `io_other_error` fixups.
    - Keep store APIs ergonomic for callers and tests.
  - Validation: `cargo clippy --offline`; `cargo test --offline`

- [x] T016: Clippy cleanup — TUI (`manual_inspect`, `question_mark`, `iter_kv_map`) (owner: mayor) (scope: src/tui/mod.rs) (depends: -)
  - Completed_at: 2026-02-07T22:54:11+00:00
  - Completion_note: clippy/test green (cargo clippy --offline: 0 warnings; cargo test --offline: 182 passed)
  - Started_at: 2026-02-07T22:08:29+00:00
  - DoD:
    - `cargo clippy --offline` emits no warnings originating from `src/tui/mod.rs`.
    - Avoid per-frame allocation regressions while applying the refactors (preserve behavior).
  - Validation: `cargo clippy --offline`; `cargo test --offline`

- [x] T002: Make walkthrough deletion representable (no resurrection from stale files) (owner: mayor) (scope: src/store/) (depends: spec:09-session-store/T006,spec:10-walkthroughs/T001)
  - Completed_at: 2026-02-07T23:21:49+00:00
  - Completion_note: store now persists walkthrough_ids + restores walkthrough rev in O(1) with cap; tests green (cargo test --offline: 185 passed)
  - Started_at: 2026-02-07T22:57:47+00:00
  - Context (from `docs/audit.md`): `save_session` never deletes old `walkthroughs/*.wt.json`, and `load_session` loads all `*.wt.json` it finds.
	- DoD:
	    - Extend `nereid-session.meta.json` to include a `walkthrough_ids` list (backwards compatible via `#[serde(default)]`).
    - On save:
      - Persist `walkthrough_ids` from the in-memory session.
      - Ensure walkthrough export still uses atomic writes.
      - Optional but preferred: garbage-collect stale walkthrough files (`*.wt.json` + `*.ascii.txt`) that are not referenced by `walkthrough_ids`.
    - On load:
      - If `walkthrough_ids` is present, load exactly those walkthroughs (no directory scan resurrection).
      - If absent, keep legacy directory-scan behavior for backwards compatibility.
    - Unit tests prove a walkthrough removed from the in-memory session does not reappear after save→load.
  - Validation: `cargo test --offline`
  - Escalate if: this requires protocol changes outside store; keep it store-local + backwards compatible.

- [x] T003: Restore walkthrough `rev` in O(1) with safety cap (owner: mayor) (scope: src/model/,src/store/) (depends: spec:10-walkthroughs/T001)
  - Completed_at: 2026-02-07T23:21:49+00:00
  - Completion_note: store now persists walkthrough_ids + restores walkthrough rev in O(1) with cap; tests green (cargo test --offline: 185 passed)
  - Started_at: 2026-02-07T22:57:47+00:00
  - Context (from `docs/audit.md`): `walkthrough_from_json` restores `rev` by looping `bump_rev()`; hostile values can cause slow loads.
  - DoD:
    - Add a direct rev setter on the model (e.g. `Walkthrough::set_rev(u64)`) mirroring `Diagram::set_rev`.
    - Update store load to set `rev` directly (no loop), and reject/cap pathological `rev` values.
    - Unit tests cover:
      - round-trip preserves `rev`
      - pathological `rev` is rejected/capped deterministically
  - Validation: `cargo test --offline`
  - Escalate if: changing model invariants is required; keep setter minimal and internal to store/tests if possible.
