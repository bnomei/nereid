# 05 - Attention Spotlight With Follow-AI

## Metadata
- `id`: `PB-05`
- `goal`: verify collaboration-state tools for spotlight handoff to the human UI.
- `session`: `data/demo-session` (use `--demo`).
- `difficulty`: `intermediate`
- `mutates_state`: `yes` (UI collaboration state)

## Setup
1. Start Nereid in demo TUI mode (serves MCP HTTP): `cargo run -- --demo`.
2. Keep the TUI running so `ui_state` is live.
3. Connect your AI client to MCP HTTP (`http://127.0.0.1:27435/mcp` unless overridden).
4. Start from a fresh prompt context.

## User Prompt
`Enable follow-AI and spotlight d:om-12-sharks/seq/participant/p:mako. Then confirm follow_ai and current agent attention.`

## Expected Tool Calls
### Required (order matters)
1. `follow_ai.set`
   - matcher: `enabled` `equals` `true`
2. `attention.agent.set`
   - matcher: `object_ref` `equals` `d:om-12-sharks/seq/participant/p:mako`
3. `attention.agent.read`
4. `follow_ai.read`

### Optional (acceptable alternatives)
- Initial `follow_ai.read` before `follow_ai.set`.
- `attention.human.read` for additional context.

### Forbidden
- `diagram.apply_ops`
- `diagram.propose_ops`
- `diagram.create_from_mermaid`
- `selection.update`
- `xref.add`
- `xref.remove`
- `walkthrough.apply_ops`

## Expected Assistant Output
- Must report `follow_ai` as enabled (`true`).
- Must report attention object ref as `d:om-12-sharks/seq/participant/p:mako`.
- Should mention diagram id `om-12-sharks`.

## Pass/Fail Checklist
- [ ] `follow_ai.set(true)` was called.
- [ ] `attention.agent.set` targeted the exact mako object ref.
- [ ] Readback calls confirm both follow-AI and agent attention state.
- [ ] Final answer accurately reflects readback state.

## Notes
- Run this playbook in TUI mode, not `--mcp` stdio mode, so follow-AI behavior is visible.
- This playbook intentionally mutates UI collaboration state; reset manually if needed.
