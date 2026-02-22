# 13 - Session State, View, Selection

## Metadata
- `id`: `PB-13`
- `goal`: verify session state tools that report active diagram, view state, and selection.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Ensure the session starts fresh with no selection (restart demo session if needed).

## User Prompt
`Without changing anything, tell me the active diagram id and the number of selected objects.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.current`
2. `view.read_state`
3. `selection.read`

### Optional (acceptable alternatives)
- `diagram.list` for orientation.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must include active diagram id `demo-00-index`.
- Must state selected object count is `0`.

## Pass/Fail Checklist
- [ ] Required tool calls happened in order.
- [ ] Output includes active diagram id and selection count.
- [ ] No forbidden mutating calls were made.

## Notes
- If selection is not empty, reset the demo session and retry.
