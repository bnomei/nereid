# 18 - Walkthrough Overview

## Metadata
- `id`: `PB-18`
- `goal`: cover walkthrough list, open, current, read, stat, and render.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`List walkthroughs, open wt-demo, then report current walkthrough id, node/edge counts, and a short render preview.`

## Expected Tool Calls
### Required (order matters)
1. `walkthrough.list`
2. `walkthrough.open`
   - matcher: `walkthrough_id` `equals` `wt-demo`
3. `walkthrough.current`
4. `walkthrough.read`
   - matcher: `walkthrough_id` `equals` `wt-demo`
5. `walkthrough.stat`
   - matcher: `walkthrough_id` `equals` `wt-demo`
6. `walkthrough.render_text`
   - matcher: `walkthrough_id` `equals` `wt-demo`

### Optional (acceptable alternatives)
- `walkthrough.get_node` for spot checks.

### Forbidden
- `walkthrough.apply_ops`
- `diagram.apply_ops`
- `xref.add`
- `xref.remove`

## Expected Assistant Output
- Must include `wt-demo` as current walkthrough id.
- Must report node count `2` and edge count `1`.
- Must include a non-empty render preview.

## Pass/Fail Checklist
- [ ] All required walkthrough tools were called in order.
- [ ] Output includes current id, counts, and render preview.
- [ ] No forbidden mutating calls were made.

## Notes
- Walkthrough fixtures live under `data/demo-session/walkthroughs`.
