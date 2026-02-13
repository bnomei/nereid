# Design — 19-mcp-protocol-surface

Implement missing tools in `src/mcp/server.rs` and define their typed params/responses in `src/mcp/types.rs`.

Notes:
- `diagram.get_ast` must return a JSON-friendly DTO (model AST types are not directly serializable).
- `diagram.propose_ops` should reuse the existing ops apply logic by applying to a clone and returning the resulting delta without committing any state.

