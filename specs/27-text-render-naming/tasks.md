# Tasks — 27-text-render-naming

Meta:
- Spec: 27-text-render-naming — Fix “ascii” naming mismatch for Unicode text renders
- Depends on: spec:12-mcp/T015, spec:12-mcp/T012, spec:09-session-store/T004, spec:10-walkthroughs/T003
- Global scope:
  - src/mcp/
  - src/store/session_folder.rs
  - docs/

## In Progress

- (none)

## Blocked

## Todo

- (none)

## Done

- [x] T001: Decide export/tool compatibility strategy (owner: perf-agent) (scope: specs/27-text-render-naming/) (depends: -)
  - Started_at: 2026-02-08T16:46:11+00:00
  - Completed_at: 2026-02-08T16:46:47+00:00
  - Completion note: Chose strategy A (keep writing `*.ascii.txt` as a legacy filename, add `*.render_text` tools, and update naming/docs for Unicode text output).
  - Validation result: n/a (decision)

- [x] T002: Add MCP tools `diagram.render_text` and `walkthrough.render_text` (owner: perf-agent) (scope: src/mcp/types.rs, src/mcp/server.rs) (depends: T001)
  - Started_at: 2026-02-08T16:46:47+00:00
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Added MCP tools as thin wrappers around the Unicode renderers and added unit tests asserting parity with the legacy `render_ascii` tools; updated `ServerInfo.instructions`.
  - Validation result: `cargo test --offline` (ok)

- [x] T003: Align session export naming with “text” terminology (owner: perf-agent) (scope: src/store/session_folder.rs, docs/) (depends: T001)
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Kept legacy `*.ascii.txt` exports but updated store code/comments/tests to treat them as deterministic text renders (Unicode allowed); protocol docs already describe `.ascii.txt` as a legacy filename.
  - Validation result: `cargo test --offline` (ok)

- [x] T004: Add regression tests for tool/export parity (owner: perf-agent) (scope: src/mcp/server.rs, src/store/session_folder.rs) (depends: T002,T003)
  - Completed_at: 2026-02-08T17:45:35+00:00
  - Completion note: Added parity tests ensuring `render_text` output matches the legacy `render_ascii` output.
  - Validation result: `cargo test --offline` (ok)
