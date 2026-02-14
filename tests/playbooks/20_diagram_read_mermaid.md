# 20 - Mermaid Source Read

## Metadata
- `id`: `PB-20`
- `goal`: require a raw Mermaid read and pinpoint a specific line in the source.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Show me the raw Mermaid source for the flowchart demo with alpha/beta edges, and point to the exact line that encodes the beta edge.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.read`
   - matcher: `diagram_id` `equals` `demo-flow`

### Optional (acceptable alternatives)

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must include the Mermaid line `a -->|beta| c(C)`.
- Must identify the diagram as `demo-flow`.

## Pass/Fail Checklist
- [ ] `diagram.read` was used for Mermaid source retrieval.
- [ ] Output includes the beta line exactly.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook validates literal Mermaid source access, not AST-derived summaries.
