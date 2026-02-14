# 19 - Walkthrough Mutation And Diff

## Metadata
- `id`: `PB-19`
- `goal`: cover walkthrough apply, diff, and get_node with a deterministic mutation.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `yes`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`On walkthrough wt-demo, add a node n:wrap titled "Wrap up" that references the Sail home node in the return diagram, then show the diff since the previous rev and read back that node.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.get_ast`
   - matcher: `diagram_id` `equals` `om-13-return`
3. `walkthrough.stat`
   - matcher: `walkthrough_id` `equals` `wt-demo`
4. `walkthrough.apply_ops`
   - matcher: `walkthrough_id` `equals` `wt-demo`
   - matcher: `base_rev` `equals` value returned by `walkthrough.stat`
5. `walkthrough.diff`
   - matcher: `walkthrough_id` `equals` `wt-demo`
6. `walkthrough.get_node`
   - matcher: `walkthrough_id` `equals` `wt-demo`
   - matcher: `node_id` `equals` `n:wrap`

### Optional (acceptable alternatives)
- `walkthrough.read` for full confirmation.

### Forbidden
- `diagram.apply_ops`
- `xref.add`
- `xref.remove`

## Expected Assistant Output
- Must report that node `n:wrap` was added.
- Must include node title `Wrap up`.
- Must include ref `d:om-13-return/flow/node/n:sail_home`.

## Pass/Fail Checklist
- [ ] `walkthrough.apply_ops` was called with the current base rev.
- [ ] `walkthrough.diff` shows an added node.
- [ ] `walkthrough.get_node` returns the new node details.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook mutates the demo walkthrough; restart demo mode afterward if you need a clean state.
