# Tasks — 20-walkthrough-mcp-mutation

Meta:
- Spec: 20-walkthrough-mcp-mutation — Walkthrough delta + mutation MCP tools
- Depends on: spec:19-mcp-protocol-surface/T003, spec:10-walkthroughs/T003, spec:18-mcp-writeback/T001
- Global scope:
  - src/mcp/

## In Progress

## Blocked

## Todo

## Done

- [x] T001: Add `walkthrough.apply_ops` + `walkthrough.get_delta` (+ persistence in `--mcp --session`) (owner: worker:019c3d0e-8309-7ae2-b3f7-33f0ea1eb071) (scope: src/mcp/server.rs, src/mcp/types.rs) (depends: -)
  - Started_at: 2026-02-08T11:39:07+00:00
  - Completed_at: 2026-02-08T11:56:24+00:00
  - Completion note: Added MCP tools `walkthrough.apply_ops` (base_rev-gated structured walkthrough mutations with stable delta refs) and `walkthrough.get_delta` (bounded delta history window), updated `ServerInfo.instructions`, and implemented persistent-mode writeback for walkthrough mutations by saving the session folder before committing server state; added unit tests for conflicts, delta spans, and persistence.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 204 passed)
