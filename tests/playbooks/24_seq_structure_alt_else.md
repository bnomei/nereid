# 24 - Sequence Structure Alt/Else Ops

## Metadata
- `id`: `PB-24`
- `goal`: add an `alt`/`else` block with two messages via structured sequence ops.
- `session`: empty/temp session (create diagram first) or any MCP session with write access.
- `difficulty`: `advanced`
- `mutates_state`: `yes`

## Setup
1. Start MCP: `cargo run -- --mcp` (or TUI + HTTP MCP).
2. Connect your AI client to this MCP server.
3. Use a fresh prompt context.

## User Prompt
`Create a temporary sequence diagram d-struct-play with participants A and B, then wrap two messages in an alt/else block: main "cache" message "hit", else "miss" message "miss". Confirm the block structure with object_read or get_ast.`

## Expected Tool Calls
### Required (order matters)
1. `diagram_create_from_mermaid`
   - matcher: mermaid `contains` `sequenceDiagram`
   - matcher: `diagram_id` `equals` `d-struct-play` (or equivalent unique id)
2. `diagram_stat`
   - matcher: active/target diagram is the new sequence diagram
3. `diagram_propose_ops` (optional but preferred) then `diagram_apply_ops`
   - matcher: ops include `seq_add_block` with `kind` `alt`
   - matcher: ops include `seq_add_section` with `kind` `else`
   - matcher: ops include `seq_add_message` with `section_id` for main and else
4. `diagram_get_ast` or `object_read`
   - matcher: block ref under `seq/block`
   - matcher: sections list main+else message membership

### Optional (acceptable alternatives)
- Create participants/messages first with flat ops, then `seq_add_block` + `seq_set_message_section`.
- `diagram_diff` after apply.
- `attention_agent_set` on the new block.

### Forbidden
- Rewriting the whole diagram via a second `diagram_create_from_mermaid` instead of structure ops (unless create was only for bootstrap).
- Leaving empty sections in the final applied state.

## Expected Assistant Output
- Must report new_rev after apply.
- Must mention block id and that main/else sections hold `hit` / `miss` (or the chosen texts).
- Must not claim structure succeeded if apply returned invalid structure errors.

## Pass/Fail Checklist
- [ ] Required structure ops were applied with a valid `base_rev`.
- [ ] Final AST has one `alt` block with main and else sections each containing one message.
- [ ] Messages are contiguous in `order_key` order across sections.
- [ ] No forbidden bulk rewrite was used for the structure step.

## Notes
- Section messages must stay contiguous in global message order; empty sections fail validation.
- Restart the session afterward if you need a clean demo state.
