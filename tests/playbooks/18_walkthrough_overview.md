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
1. `walkthrough_list`
2. `walkthrough_open`
   - matcher: `walkthrough_id` `equals` `wt-demo`
3. `walkthrough_current`
4. `walkthrough_read`
   - matcher: `walkthrough_id` `equals` `wt-demo`
5. `walkthrough_stat`
   - matcher: `walkthrough_id` `equals` `wt-demo`
6. `walkthrough_render_text`
   - matcher: `walkthrough_id` `equals` `wt-demo`

### Optional (acceptable alternatives)
- `walkthrough_get_node` for spot checks.

### Forbidden
- `walkthrough_apply_ops`
- `diagram_apply_ops`
- `xref_add`
- `xref_remove`

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
