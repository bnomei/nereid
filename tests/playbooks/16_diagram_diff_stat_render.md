# 16 - Diagram Diff, Stat, Render

## Metadata
- `id`: `PB-16`
- `goal`: cover `diagram_diff`, `diagram_stat`, and `diagram_render_text` in one flow.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `yes` (creates and updates a temporary diagram)

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Create a new sequence diagram pb-16-seq with a->>b: Ping, then add another message a->>b: Extra, and report the diff, current counts, and a rendered text preview.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_create_from_mermaid`
   - matcher: `diagram_id` `equals` `pb-16-seq`
2. `diagram_apply_ops`
   - matcher: `diagram_id` `equals` `pb-16-seq`
   - matcher: `base_rev` `equals` `0`
3. `diagram_diff`
   - matcher: `diagram_id` `equals` `pb-16-seq`
   - matcher: `since_rev` `equals` `0`
4. `diagram_stat`
   - matcher: `diagram_id` `equals` `pb-16-seq`
5. `diagram_render_text`
   - matcher: `diagram_id` `equals` `pb-16-seq`

### Optional (acceptable alternatives)
- `diagram_read` before or after apply.

### Forbidden
- `diagram_delete`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must indicate that diff contains changes since rev `0`.
- Must report message count `2` in the diagram stat.
- Must include a non-empty render preview.

## Pass/Fail Checklist
- [ ] All required diagram tools were called in order.
- [ ] Output includes diff summary, stat counts, and render preview.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook mutates the demo session; restart demo mode afterward if you need a clean state.
