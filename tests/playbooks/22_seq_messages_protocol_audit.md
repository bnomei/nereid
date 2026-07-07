# 22 - Protocol Audit (Messages Between Speakers)

## Metadata
- `id`: `PB-22`
- `goal`: require `seq_messages` using human-sounding intent and return exact message refs + text.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `advanced`
- `mutates_state`: `no`

## Setup
1. Start MCP in demo mode: `cargo run -- --demo --mcp`.
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Do a quick protocol audit of the terrace dialogue: list only the lines spoken by the boy to the old man, returning message refs and text.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_list`
2. `diagram_get_ast`
   - matcher: `diagram_id` `equals` `om-10-dialogue`
3. `seq_messages`
   - matcher: `diagram_id` `equals` `om-10-dialogue`
   - matcher: `from_participant_id` `equals` `p:manolin`
   - matcher: `to_participant_id` `equals` `p:santiago`
4. `object_read`
   - matcher: reads `d:om-10-dialogue/seq/message/m:ask_go`
   - matcher: reads `d:om-10-dialogue/seq/message/m:yankees`

### Optional (acceptable alternatives)
- `seq_search` as a sanity check (must still use `seq_messages`).

### Forbidden
- `diagram_apply_ops`
- `diagram_propose_ops`
- `diagram_create_from_mermaid`
- `selection_update`
- `xref_add`
- `xref_remove`
- `walkthrough_apply_ops`

## Expected Assistant Output
- Must include `d:om-10-dialogue/seq/message/m:ask_go` with text `Can I go with you again`.
- Must include `d:om-10-dialogue/seq/message/m:yankees` with text `The Yankees cannot lose`.

## Pass/Fail Checklist
- [ ] `seq_messages` was used with the correct from/to participants.
- [ ] `object_read` was used to report exact text.
- [ ] Output includes the two expected refs and texts only.
- [ ] No forbidden mutating calls were made.

## Notes
- The prompt is intentionally human-sounding; the agent must infer the correct diagram and participants.
