# 17 - Diagram Delete

## Metadata
- `id`: `PB-17`
- `goal`: validate diagram deletion and list refresh.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `yes` (creates and deletes a temporary diagram)

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Create a temporary flowchart diagram pb-17-flow, then delete it and confirm it is gone from diagram.list.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.create_from_mermaid`
   - matcher: `diagram_id` `equals` `pb-17-flow`
2. `diagram.delete`
   - matcher: `diagram_id` `equals` `pb-17-flow`
3. `diagram.list`

### Optional (acceptable alternatives)
- `diagram.current` after delete.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must state that `pb-17-flow` was deleted.
- Must confirm `pb-17-flow` is not present in `diagram.list`.

## Pass/Fail Checklist
- [ ] `diagram.create_from_mermaid`, `diagram.delete`, and `diagram.list` were called in order.
- [ ] Output confirms deletion and absence from list.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook mutates the demo session; restart demo mode afterward if you need a clean state.
