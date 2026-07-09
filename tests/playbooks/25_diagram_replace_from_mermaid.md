# 25 - Diagram Replace From Mermaid Identity

## Metadata
- `id`: `PB-25`
- `goal`: replace a sequence diagram from Mermaid while preserving matching message ids and reporting identity.
- `session`: temporary MCP session (create diagram first).
- `difficulty`: `advanced`
- `mutates_state`: `yes`

## Setup
1. Start MCP: `cargo run -- --mcp`.
2. Connect your AI client.
3. Fresh prompt context.

## User Prompt
`Create sequence diagram d-replace-play with A and B and one message "Hello" using create_from_mermaid or ops. Note the message object id. Then call diagram_replace_from_mermaid with equivalent Mermaid that keeps the same "Hello" text, and report which object ids were preserved. Finally replace again with text "Changed" and report newly allocated / dropped ids.`

## Expected Tool Calls
### Required (order matters)
1. Bootstrap diagram (`diagram_create_from_mermaid` or structure/ops path).
2. `diagram_stat` or `diagram_get_ast` to capture current rev and message id.
3. `diagram_replace_from_mermaid`
   - matcher: `base_rev` equals current rev
   - matcher: mermaid contains `Hello`
4. Read identity from replace response (`identity.preserved` should include prior message id when fingerprint matches).
5. `diagram_replace_from_mermaid` again with changed text and higher `base_rev`.
6. Confirm `identity.dropped` / `identity.newly_allocated` reflect the rename.

### Optional
- `xref_add` before rename to observe `dangling_xref_ids` after text change.
- `diagram_propose_ops` is not required for replace.

### Forbidden
- Using `diagram_create_from_mermaid` as a substitute for replace on the same diagram_id (create rejects existing ids).

## Expected Assistant Output
- Must report preserved message id after the identity-preserving replace.
- Must report that changing message text dropped the old id / allocated a new one.
- Must not claim kind mismatch when both sources are sequence diagrams.

## Pass/Fail Checklist
- [ ] Replace used current `base_rev`.
- [ ] First replace preserved matching message id.
- [ ] Second replace reported identity change for renamed text.
- [ ] No create-with-same-id was used for the replace steps.

## Notes
- Message fingerprint is `(from, to, kind, text)` after participant remapping.
- Structure ops remain preferred for local block edits; replace is the bulk-rewrite escape hatch.
