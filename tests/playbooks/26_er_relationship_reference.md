# 26 - ER Relationship Reference

## Metadata
- `id`: `PB-26`
- `goal`: resolve an ER relationship as a first-class object ref and inspect its local endpoints.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`In the ER demo, inspect the relationship labeled "places". Return its relationship object_ref, exact endpoint entity refs, and Mermaid connector, then show its local slice.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_get_ast`
   - matcher: `diagram_id` `equals` `demo-er`
2. `object_read`
   - matcher: `object_ref` `equals` `d:demo-er/er/relationship/r:0001`
3. `diagram_get_slice`
   - matcher: `diagram_id` `equals` `demo-er`
   - matcher: `center_ref` `equals` `d:demo-er/er/relationship/r:0001`

### Optional (acceptable alternatives)
- `diagram_read` to confirm the exact Mermaid relationship line.
- `attention_agent_set` to spotlight the relationship in a live TUI.

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_replace_from_mermaid`
- `diagram_create_from_mermaid`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include relationship ref `d:demo-er/er/relationship/r:0001`.
- Must include endpoint refs `d:demo-er/er/entity/e:CUSTOMER` and `d:demo-er/er/entity/e:ORDER`.
- Must report connector `||--o{` and label `places`.
- Must not describe the relationship as a flowchart diagram.

## Pass/Fail Checklist
- [ ] Required tool calls happened in order.
- [ ] Output includes the relationship ref, both entity refs, connector, and label.
- [ ] The local slice includes the relationship and both endpoints.
- [ ] No forbidden mutating calls were made.
- [ ] No hallucinated IDs, entities, or tools appear.

## Notes
- `object_read` returns typed ER relationship fields; the canonical category is `er/relationship`.
