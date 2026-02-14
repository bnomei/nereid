# 12 - Ambiguous OK In Else Branch

## Metadata
- `id`: `PB-12`
- `goal`: disambiguate repeated message text using block AST and return the correct message ref.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`In the ambiguous OK demo, multiple messages say "OK". Return the object_ref for the OK message in the "cache miss" else branch where api talks to db, and include its from/to participant IDs and message_id.`

## Expected Tool Calls
### Required (order matters)
1. `diagram.list`
2. `diagram.get_ast`
   - matcher: `diagram_id` `equals` `demo-t-seq-ambiguous`
3. `object.read`
   - matcher: reads `d:demo-t-seq-ambiguous/seq/message/m:ok_db`

### Optional (acceptable alternatives)
- `seq.search` with `needle` `equals` `OK` (must still use AST to pick the else-branch message).

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must include message ref `d:demo-t-seq-ambiguous/seq/message/m:ok_db`.
- Must include `message_id` `m:ok_db`.
- Must include `from_participant_id` `p:api` and `to_participant_id` `p:db`.
- Must mention that this is the else/cache miss branch.

## Pass/Fail Checklist
- [ ] `diagram.get_ast` was used to disambiguate the else-branch message.
- [ ] `object.read` was used to confirm from/to and id.
- [ ] Output includes the exact message ref, ids, and branch context.
- [ ] No forbidden mutating calls were made.

## Notes
- This playbook is designed to fail if the agent picks any other "OK" message.
