# Tasks — 24-mcp-query-extensions

Meta:
- Spec: 24-mcp-query-extensions — MCP query surface upgrades
- Depends on: spec:12-mcp/T023, spec:08-query-engine/T001
- Global scope:
  - src/mcp/
  - src/query/
  - Cargo.toml

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T006: Add regression tests for determinism + edge cases (owner: mayor) (scope: src/mcp/server.rs, src/query/) (depends: T001,T002,T003,T005)
  - Completed_at: 2026-02-08T18:31:51+00:00
  - Completion note: Verified regression coverage is in place: `xref.list` filter ordering + limit, `session.routes` determinism + ordering param, and `seq.search` regex compile errors + case-insensitivity (tests added across T001/T002/T003/T005); no additional harness changes needed.
  - Validation result: `cargo test --offline` (ok, 251 passed)

- [x] T003: Add regex + case-insensitive support to `seq.search` (owner: worker:019c3e79-cc37-7193-92e6-195e8a30e1d1) (scope: Cargo.toml, Cargo.lock, src/mcp/types.rs, src/mcp/server.rs, src/query/sequence.rs) (depends: -)
  - Started_at: 2026-02-08T18:17:46+00:00
  - Completed_at: 2026-02-08T18:30:34+00:00
  - Completion note: Added `regex` support to `seq.search` via params `mode=substring|regex` (default: `substring`) and `case_insensitive` (default: `true`); compile regex once per call; invalid `mode`/regex map to `invalid_params`; added unit tests in query + MCP server.
  - Validation result: `cargo test --offline` (ok, 251 passed)

- [x] T005: Add `flow.degrees` fan-in/fan-out stats tool (owner: mayor) (scope: src/query/flow.rs, src/mcp/types.rs, src/mcp/server.rs) (depends: -)
  - Started_at: 2026-02-08T18:03:56+00:00
  - Completed_at: 2026-02-08T18:14:22+00:00
  - Completion note: Implemented bounded + deterministic `flow.degrees` MCP tool with `top` truncation and `sort_by` control (`out` default, `in`, `total`); added query helper for degree computation and MCP unit tests; updated `ServerInfo.instructions` tool list.
  - Validation result: `cargo test --offline` (ok, 244 passed)

- [x] T002: Implement multi-route `session.routes` (k-shortest simple routes) (owner: mayor) (scope: src/query/session_routes.rs, src/mcp/types.rs, src/mcp/server.rs) (depends: -)
  - Started_at: 2026-02-08T16:14:15+00:00
  - Recovered_from_owner: worker:019c3e09-3ede-74d2-8099-f54a5c56b6e6
  - Recovered_at: 2026-02-08T16:49:33+00:00
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Implemented deterministic k-shortest simple route enumeration in `query::session_routes` with output ordering control (`fewest_hops` default, optional `lexicographic`); wired MCP `session.routes` to return up to `limit` routes honoring `max_hops`; added unit tests for ordering and invalid params.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok)

- [x] T001: Extend `xref.list` filtering params + implementation (owner: worker:019c3df6-2d5d-7b60-b3a4-41f2a7d07970) (scope: src/mcp/types.rs, src/mcp/server.rs) (depends: -)
  - Started_at: 2026-02-08T15:51:39+00:00
  - Completed_at: 2026-02-08T16:08:33+00:00
  - Completion note: Extended `xref.list` with deterministic server-side filters (`status`, `kind`, endpoint filters, `label_contains`, `limit`) while keeping `dangling_only`; implemented filter → sort by `xref_id` → limit; added unit tests for filters, determinism, and truncation.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 219 passed)

- [x] T004: Decide flow fan-in/out tool shape (Option A vs B) (owner: mayor) (scope: specs/24-mcp-query-extensions/) (depends: -)
  - Completed_at: 2026-02-08T15:37:46+00:00
  - Completion note: Chose Option A (`flow.degrees`) for a small, composable fan-in/fan-out primitive with bounded output.
  - Validation result: n/a (decision task)
