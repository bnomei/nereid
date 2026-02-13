# Tasks — 19-mcp-protocol-surface

Meta:
- Spec: 19-mcp-protocol-surface — Complete remaining MCP protocol tools
- Depends on: spec:12-mcp/T023
- Global scope:
  - src/mcp/

## In Progress

## Blocked

## Todo

## Done

- [x] T003: Add `diagram.propose_ops` tool (owner: worker:019c3d04-76cc-73a2-945c-a50177d31066) (scope: src/mcp/server.rs, src/mcp/types.rs) (depends: T002)
  - Started_at: 2026-02-08T11:29:49+00:00
  - Completed_at: 2026-02-08T11:36:04+00:00
  - Completion note: Added MCP tool `diagram.propose_ops` to compute the predicted rev+delta by applying ops to a cloned diagram (no mutation of server state), updated `ServerInfo.instructions`, and added unit tests for non-mutation and delta parity with `diagram.apply_ops`.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 200 passed)

- [x] T002: Add `diagram.get_ast` tool (owner: worker:019c3cfa-e8ac-7482-9e7b-ac0d5cd2bed8) (scope: src/mcp/server.rs, src/mcp/types.rs) (depends: T001)
  - Started_at: 2026-02-08T11:19:22+00:00
  - Completed_at: 2026-02-08T11:29:09+00:00
  - Completion note: Added MCP tool `diagram.get_ast` returning a JSON-friendly AST DTO (sequence/flow variants) with deterministic ordering, updated `ServerInfo.instructions`, and added unit tests for seq/flow ordering determinism.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 198 passed)

- [x] T001: Add `seq.messages` + `flow.unreachable` tools (owner: worker:019c3ced-bda8-7092-81ca-e37ba7a7e9c5) (scope: src/mcp/server.rs, src/mcp/types.rs) (depends: -)
  - Started_at: 2026-02-08T11:04:09+00:00
  - Completed_at: 2026-02-08T11:18:34+00:00
  - Completion note: Implemented MCP tools `seq.messages` and `flow.unreachable` with deterministic ordering, strict `INVALID_PARAMS` mapping for invalid inputs and diagram kind mismatches, updated `ServerInfo.instructions`, and added unit tests for filtering/determinism and error mapping.
  - Validation: `cargo test --offline`
  - Validation result: `cargo test --offline` (ok, 196 passed)
