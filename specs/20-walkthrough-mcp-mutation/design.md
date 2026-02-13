# Design — 20-walkthrough-mcp-mutation

Implement the new walkthrough tools in `src/mcp/server.rs` with typed params/responses in `src/mcp/types.rs`.

Approach:
- Define a small walkthrough-op enum in MCP types (add/update/remove node; add/update/remove edge; set walkthrough title).
- Apply ops against a cloned `Session`/`Walkthrough` to compute the delta; bump walkthrough `rev` by 1 on non-empty ops.
- Keep a bounded delta history per walkthrough id, similar to diagram deltas.
- In persistent MCP mode, save the mutated session folder via `SessionFolder::save_session(&Session)` before committing the in-memory state.

