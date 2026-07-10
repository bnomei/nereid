---
name: nereid
description: Collaborate in Nereid Mermaid sessions via MCP using AST-first, probe-refine workflows for sequence, flowchart, class, entity-relationship, and Gantt diagrams, plus xrefs, routes, and walkthroughs. Use when exploring or editing diagrams with a human watching live in TUI, and when coordinating attention through `attention_*`, `follow_ai_*`, and `selection_*`.
---

# Nereid MCP Collaboration

Collaborate on Mermaid-backed diagrams in a live, shared TUI/MCP session. Keep context small, edits structured, and attention explicit.

## Collaboration Contract

Assume co-presence by default:
- The user can see diagram updates in real time.
- The user can see where the agent is focused.
- The agent should steer attention visually, then speak briefly.

Drive collaboration with this state model:
- `attention_human_read`: read the human cursor/attention in TUI.
- `attention_agent_read`: read the agent spotlight object.
- `attention_agent_set`: move the agent spotlight to one object.
- `attention_agent_clear`: clear the agent spotlight.
- `follow_ai_read` / `follow_ai_set`: read or toggle whether TUI follows agent spotlight.
- `selection_read` / `selection_update`: shared working-set selection (multi-object).

Treat these as separate concerns:
- Human attention: what the person is looking at.
- Agent attention: what the agent wants to emphasize now.
- Selection: short-term working set for batch reasoning/edits.
- Follow-AI: whether the TUI camera/cursor tracks agent spotlight.

## Core Principles

- Treat AST as source of truth; rendered text and Mermaid text are projections.
- Session files (`nereid-session.meta.json`, `diagrams/*.mmd`, `walkthroughs/*.wt.json`) are app-managed snapshots and can be rewritten frequently while Nereid runs.
- Use canonical `ObjectRef` everywhere: `d:<diagram_id>/<category...>/<object_id>`.
  Supported category pairs are `seq/participant`, `seq/message`, `seq/block`, `seq/section`,
  `flow/node`, `flow/edge`, `class/class`, `class/relation`, `er/entity`, `er/relationship`,
  `gantt/section`, `gantt/task`, and `gantt/lane`.
- Prefer small reads first (`diagram_stat`, `diagram_get_slice`, `diagram_diff`, `walkthrough_diff`).
- Use typed query tools (`seq_*`, `flow_*`, `xref_*`, `route_find`) before large snapshots.
- Gate edits with `base_rev` and keep ops minimal.
- Record evidence as refs (xrefs and walkthrough nodes) so reasoning is resumable.
- Keep dangling xrefs visible as TODO artifacts unless asked to clean them.

## Diagram Kind Boundaries

| Kind | Mermaid header | Canonical object categories | Mutation path |
|---|---|---|---|
| Sequence | `sequenceDiagram` | `seq/participant`, `seq/message`, `seq/block`, `seq/section` | structured `seq_*` ops or Mermaid replace |
| Flowchart | `flowchart` / `graph` | `flow/node`, `flow/edge` | structured `flow_*` ops or Mermaid replace |
| Class | `classDiagram` | `class/class`, `class/relation` | Mermaid replace |
| ER | `erDiagram` | `er/entity`, `er/relationship` | Mermaid replace |
| Gantt | `gantt` | `gantt/section`, `gantt/task`, `gantt/lane` | Mermaid replace |

Creation, raw Mermaid reads, text rendering, xrefs, selection, and attention work for all five
kinds. `diagram_get_ast` and `object_read` return kind-specific fields, including class members,
typed ER cardinalities, and Gantt sections, starts, durations, and dependencies. Typed mutation ops
and dedicated query helpers remain sequence/flow-specific, so edit class, ER, and Gantt diagrams
with Mermaid replace. `gantt/lane` refs identify rendered time headers; prefer task or section refs
for structural probing.

## Execution Discipline

- Use MCP tools as the only source of truth.
- Do not inspect `src/`, `tests/`, `docs/`, or `data/` to answer runtime collaboration questions.
- Make the first relevant MCP call immediately after reading the user prompt.
- Prefer direct MCP execution over schema/code exploration.
- If payload shape is unclear, call the tool once and adapt from validation errors.
- Use shell/file probing only when the user explicitly asks for file-level inspection or storage debugging.

## Tool Groups

- Diagram lifecycle and target: `diagram_list`, `diagram_open`, `diagram_delete`, `diagram_current`, `diagram_create_from_mermaid`
- Diagram reads: `diagram_stat`, `diagram_get_slice`, `diagram_diff`, `diagram_read`, `diagram_get_ast`, `diagram_render_text`
- Diagram mutation: `diagram_propose_ops`, `diagram_apply_ops`, `diagram_replace_from_mermaid`
- Walkthrough lifecycle and target: `walkthrough_list`, `walkthrough_open`, `walkthrough_current`
- Walkthrough reads: `walkthrough_stat`, `walkthrough_diff`, `walkthrough_read`, `walkthrough_get_node`, `walkthrough_render_text`
- Walkthrough mutation: `walkthrough_apply_ops`
- Collaboration state: `attention_human_read`, `attention_agent_read`, `attention_agent_set`, `attention_agent_clear`, `follow_ai_read`, `follow_ai_set`, `selection_read`, `selection_update`, `view_read_state`
- Cross-diagram mapping: `xref_list`, `xref_neighbors`, `xref_add`, `xref_remove`
- Object inspection: `object_read`
- Query helpers (route): `route_find`
- Query helpers (sequence): `seq_messages`, `seq_search`, `seq_trace`
- Query helpers (flow): `flow_reachable`, `flow_paths`, `flow_cycles`, `flow_unreachable`, `flow_dead_ends`, `flow_degrees`

## Default Operating Loop

1. Resolve target:
   - `diagram_current` -> `diagram_list` -> `diagram_open`.
   - `walkthrough_current` -> `walkthrough_list` -> `walkthrough_open`.
   - if no diagram exists yet, bootstrap with `diagram_create_from_mermaid`.
2. Read live collab state:
   - `attention_human_read`, `attention_agent_read`, `follow_ai_read`, `selection_read`.
3. Probe local context:
   - `diagram_stat`, `diagram_get_slice`, then one or two typed queries.
4. Steer visual attention:
   - `attention_agent_set` to the object currently being discussed.
   - keep chat short while attention marker carries micro-guidance.
5. Propose and apply minimal edits:
   - `diagram_propose_ops` -> `diagram_apply_ops`.
   - `walkthrough_apply_ops` for walkthrough refinement.
6. Refresh with deltas:
   - `diagram_diff` / `walkthrough_diff` (avoid full re-read unless needed).

## Live Co-Presence Rules

- Use spotlight-first communication: set `attention_agent_set` before explaining a local change.
- Move spotlight when changing topic; clear it when complete.
- Keep narration compact when user is actively watching the diagram.
- Skip spotlight changes for quick query-only answers unless orientation is needed.
- Use `selection_update` for temporary multi-object working sets, not as a focus proxy.
- For create/switch-only requests, stop after `diagram_create_from_mermaid` (and optional
  `attention_agent_set`); avoid extra `diagram_stat`, `diagram_render_text`, or `flow_*` probes
  unless the user asks for inspection/debugging.

## Tool Contracts (Input/Output)

### `diagram_create_from_mermaid`
Input:
```json
{
  "mermaid": "flowchart TD\n  A --> B",
  "diagram_id": "d-my-flow",
  "name": "My Flow",
  "make_active": true
}
```
Output:
```json
{
  "diagram": {
    "diagram_id": "d-my-flow",
    "name": "My Flow",
    "kind": "flowchart",
    "rev": 0
  },
  "active_diagram_id": "d-my-flow"
}
```

### `diagram_delete`
Input:
```json
{
  "diagram_id": "d-my-flow"
}
```
Output:
```json
{
  "deleted_diagram_id": "d-my-flow",
  "active_diagram_id": "d-next"
}
```

### `diagram_get_slice`
Input:
```json
{
  "diagram_id": "d-flow",
  "center_ref": "d:d-flow/flow/node/n:a",
  "radius": 1,
  "depth": 1,
  "filters": {
    "include_categories": ["flow/node", "flow/edge"],
    "exclude_categories": []
  }
}
```
Output:
```json
{
  "objects": ["d:d-flow/flow/node/n:a", "d:d-flow/flow/node/n:b"],
  "edges": ["d:d-flow/flow/edge/e:ab"]
}
```

### `diagram_apply_ops`
Input:
```json
{
  "diagram_id": "d-seq",
  "base_rev": 3,
  "ops": []
}
```
Output:
```json
{
  "new_rev": 4,
  "applied": 1,
  "delta": { "added": [], "removed": [], "updated": [] }
}
```

### `walkthrough_apply_ops`
Input:
```json
{
  "walkthrough_id": "w:1",
  "base_rev": 0,
  "ops": []
}
```
Output:
```json
{
  "new_rev": 1,
  "applied": 1,
  "delta": { "added": [], "removed": [], "updated": [] }
}
```

### `object_read`
Input:
```json
{ "object_ref": "d:d-seq/seq/block/b:0000" }
```
Output:
```json
{
  "objects": [
    {
      "object_ref": "d:d-seq/seq/block/b:0000",
      "object": {
        "type": "seq_block",
        "kind": "alt",
        "header": "guard",
        "section_ids": ["sec:0000:00", "sec:0000:01"],
        "child_block_ids": []
      }
    },
    {
      "object_ref": "d:d-seq/seq/section/sec:0000:00",
      "object": {
        "type": "seq_section",
        "kind": "main",
        "header": "ok",
        "message_ids": ["m:0000"]
      }
    }
  ],
  "context": {}
}
```

### `attention_human_read`
Input:
```json
{}
```
Output:
```json
{
  "object_ref": "d:d-auth-flow/flow/node/n:authorize",
  "diagram_id": "d-auth-flow"
}
```

### `attention_agent_read`
Input:
```json
{}
```
Output:
```json
{
  "object_ref": "d:d-auth-flow/flow/node/n:authorize",
  "diagram_id": "d-auth-flow"
}
```

### `attention_agent_set`
Input:
```json
{
  "object_ref": "d:d-auth-flow/flow/node/n:authorize"
}
```
Output:
```json
{
  "object_ref": "d:d-auth-flow/flow/node/n:authorize",
  "diagram_id": "d-auth-flow"
}
```

### `attention_agent_clear`
Input:
```json
{}
```
Output:
```json
{
  "cleared": 1
}
```

### `follow_ai_read`
Input:
```json
{}
```
Output:
```json
{
  "enabled": true
}
```

### `follow_ai_set`
Input:
```json
{
  "enabled": true
}
```
Output:
```json
{
  "enabled": true
}
```

### `selection_update`
Input:
```json
{
  "object_refs": [
    "d:d-auth-flow/flow/node/n:start",
    "d:d-auth-flow/flow/node/n:authorize"
  ],
  "mode": "replace"
}
```
Output:
```json
{
  "applied": [
    "d:d-auth-flow/flow/node/n:authorize",
    "d:d-auth-flow/flow/node/n:start"
  ],
  "ignored": []
}
```

## Probe-and-Refine on Charts

Use a shallow, typed exploration loop:
1. Anchor on one object (`attention_human_read` or explicit `object_ref`).
2. Pull local structure with `diagram_get_slice`.
3. Ask one typed question (`seq_*`, `flow_*`, `xref_*`, `route_find`).
4. Shift agent spotlight to next object if needed.
5. Repeat until ambiguity is resolved.

Escalate to global reads (`diagram_read`, `diagram_get_ast`, `diagram_render_text`) only when local probes are insufficient.

## Mutation Discipline

- Use `diagram_propose_ops` before `diagram_apply_ops` for non-trivial edits.
- Keep op batches minimal and scoped to one local intent.
- Use stable IDs for all new objects.
- Re-read `diagram_stat` or `diagram_diff` after apply to confirm resulting rev/state.

### Sequence structure ops

Prefer structured ops over rewriting Mermaid when editing `alt` / `opt` / `loop` / `par`:

| Op `type` | Purpose |
|---|---|
| `seq_add_block` | Create block + main section (`kind`: `alt`/`opt`/`loop`/`par`) |
| `seq_update_block` | Patch block header |
| `seq_remove_block` | Remove block tree; messages remain |
| `seq_add_section` | Add `else` (under `alt`) or `and` (under `par`) |
| `seq_update_section` | Patch section header |
| `seq_remove_section` | Remove empty non-main section |
| `seq_set_message_section` | Attach/move/detach message membership (`section_id: null` detaches) |
| `seq_add_message` | Optional `section_id` to place message on create |

Rules agents must follow:
- Batch create block + sections + messages with contiguous `order_key` order in one `diagram_apply_ops`.
- Messages in a section must be contiguous in global message order; empty sections are invalid.
- Nested blocks need `parent_block_id` and must sit inside one parent section’s message range.
- After structural edits, confirm with `diagram_get_ast` or `object_read` on `seq/block` / `seq/section` refs (and `diagram_read` for Mermaid with blocks).

Example batch (add `alt`/`else` with two messages):
```json
{
  "diagram_id": "d-seq",
  "base_rev": 0,
  "ops": [
    {
      "type": "seq_add_block",
      "block_id": "b:cache",
      "kind": "alt",
      "header": "cache",
      "main_section_id": "sec:cache:main"
    },
    {
      "type": "seq_add_section",
      "section_id": "sec:cache:else",
      "block_id": "b:cache",
      "kind": "else",
      "header": "miss"
    },
    {
      "type": "seq_add_message",
      "message_id": "m:hit",
      "from_participant_id": "p:A",
      "to_participant_id": "p:B",
      "kind": "sync",
      "text": "hit",
      "order_key": 1000,
      "section_id": "sec:cache:main"
    },
    {
      "type": "seq_add_message",
      "message_id": "m:miss",
      "from_participant_id": "p:A",
      "to_participant_id": "p:B",
      "kind": "sync",
      "text": "miss",
      "order_key": 2000,
      "section_id": "sec:cache:else"
    }
  ]
}
```

## Walkthrough and Evidence Artifacts

Build walkthroughs as resumable breadcrumbs:
- Add concise nodes with evidence refs (`refs`).
- Keep node titles short; put detail in `body_md`.
- Link nodes incrementally as understanding grows.
- Update walkthroughs after edits instead of re-deriving from scratch.

Use xrefs to preserve cross-diagram semantics:
- `xref_add` for implementation/expansion links.
- `xref_list` and `xref_neighbors` for map and traversal.
- Surface dangling xrefs explicitly for follow-up.

## Conflict Recovery

When mutation fails due to stale `base_rev`:
1. call `diagram_diff` or `walkthrough_diff` from last known rev,
2. rebase ops on `current_rev`,
3. retry with updated `base_rev`.

If diff history window is exhausted, fetch a fresh snapshot (`diagram_read` or `walkthrough_read`) and resume delta-first flow.

### Bulk Mermaid replace

Use `diagram_replace_from_mermaid` when a bulk rewrite is easier than structure ops:

```json
{
  "diagram_id": "d-seq",
  "base_rev": 3,
  "mermaid": "sequenceDiagram\n  A->>B: Hello\n"
}
```

Behavior:
- Kind must match the existing diagram (sequence stays sequence).
- Stable ids are reconciled from the previous AST (messages by fingerprint, blocks/sections by structure fingerprint, participants by name, flow nodes/edges similarly).
- Response includes `identity.preserved` / `identity.dropped` / `identity.newly_allocated` and `dangling_xref_ids`.
- Prefer structure ops for local alt/else/loop edits; use replace for full-source rewrites that should keep identity when content fingerprints still match.
- Renaming message text typically allocates a new message id and may dangle xrefs that pointed at the old id.

## Reporting Discipline

Report only what the user needs:
- include target (`diagram_id`/`walkthrough_id`) and revision movement (`base_rev` -> `new_rev`),
- include concise deltas and key refs,
- avoid full AST/text dumps unless requested,
- in live sessions, avoid narrating every micro-step already visible in TUI.

## MCP Playbook Companion

Use `references/mcp-playbooks.md` for extra payload templates. Keep this file as the primary protocol and behavior contract.
