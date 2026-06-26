DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/tui/mod.rs:701-712,773-789 | Slug: human-attention-stale-follow-ai

# UiState human attention stays stale while follow-AI tracks agent spotlight

## Finding

When follow-AI is enabled and the TUI viewport follows an agent spotlight, `publish_focus_to_ui_state` skips updating `human_active_diagram_id` and `human_active_object_ref` because `focus_owner == Agent`. MCP `attention.human.read` reads those fields as human attention.

## Violated Invariant Or Contract

While the human is viewing the agent-followed target, `UiState` human attention fields should reflect the visible viewport, or `attention.human.read` must document that follow-AI mode reports pre-follow selection.

## Oracle

`follow_agent_highlight` sets `focus_owner = FocusOwner::Agent` and calls `jump_to_object_ref` (`785-787`). `publish_focus_to_ui_state` only calls `set_human_selection` when `focus_owner == Human` (`708-711`). `attention.human.read` returns `context.human_active_*` (`collaboration.rs:27-30`).

## Counterexample

1. Human selects object A on diagram `d-a`; UiState publishes `{d-a, A}`. 2. Agent sets spotlight on object B on `d-b`; follow-AI on. 3. TUI jumps to B; viewport shows B. 4. UiState still reports A/`d-a`. 5. `attention.human.read` returns A while TUI displays B.

## Why It Might Matter

Agents coordinating with the human misread attention during the normal follow-AI workflow, causing edits or explanations aimed at the wrong object.

## Proof

Cross-entry mismatch: TUI session selection and diagram state update via `jump_to_object_ref`, but `UiState` human fields frozen under agent focus ownership.

## Counterevidence Checked

`attention.agent.read` exposes agent spotlight separately. TUI follow-AI tests assert viewport jump but not UiState sync (`tui/tests.rs:773-783`).

## Suggested Next Step

Publish human selection on follow-AI jumps, or add a distinct "visible" attention channel separate from ownership.

## Status Notes

2026-06-26: Marked fixed after static validation. `publish_focus_to_ui_state` now publishes visible human attention when either the human owns focus or follow-AI is enabled. Follow-AI jumps through `select_object_ref`, which publishes after selecting the followed target, so `attention.human.read` reflects the viewport during follow-AI instead of the pre-follow selection.

DEVANA-KEY: src/tui/mod.rs:701-712 | P2 | human-attention-stale-follow-ai
DEVANA-SUMMARY: fixed P2 high src/tui/mod.rs:701-712 - follow-AI viewport follows agent spotlight but UiState human attention fields are not updated, stale for MCP.
