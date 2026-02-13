# Design — DRAFT-37-inline-notes

## Overview

Protocol 01 recommends notes/annotations as a high-value discussion tool. This spec implements a **simplified** note model:
- Notes are attached to **flow nodes** and **sequence participants** (not timeline events).
- Notes are rendered inline in the box beneath the label when enabled.
- A diagram-view toggle (`n`) controls whether notes are shown.

This keeps notes:
- stable (tied to stable object ids)
- queryable (in AST)
- non-invasive (not part of `.mmd` export)

## Model changes

### Flow

Extend `FlowNode` with:
- `note: Option<String>`

### Sequence

Extend `SequenceParticipant` with:
- `note: Option<String>`

Notes are single-line strings; rendering clips at box width.

## Persistence

Notes must round-trip via session persistence, not `.mmd`.

Proposed storage:
- Extend the diagram meta sidecar JSON (or session meta) to include optional note fields for:
  - flow nodes (by stable `ObjectId` or stable mermaid id mapping)
  - sequence participants (by stable `ObjectId`)

This aligns with Protocol 01’s “AST is source of truth; `.mmd` is interchange”.

## Rendering + toggle

### Render option plumbing

Introduce a small render config:

- `RenderOptions { show_notes: bool }`

Plumb through:
- `render_diagram_unicode(_annotated)` → diagram-kind renderers
- `render_flowchart_unicode(_annotated)`
- `render_sequence_unicode(_annotated)`

### Geometry rule

When `show_notes == true`, render object boxes with an extra interior line:
- box height becomes 4 (top border + label line + note line + bottom border)

When `show_notes == false`, keep current baseline height (3).

Sequence participant header alignment:
- Always use the same participant box height within a diagram render; with notes enabled, participants without notes render a blank note line.

Flow node sizing:
- Keep layout algorithm unchanged; only renderer’s box height changes.
  - Connectors can continue to anchor to the label line (`y = box_y0 + 1`) for determinism.

### TUI keybinding

`n` is currently used for “search next” even when search mode is inactive.

Implementation rule:
- Only handle `n` as “toggle notes” when:
  - `search_mode == Inactive`, and
  - `focus == Diagram`
- Keep existing `n/N` behavior when search mode is `Results`.

Update the help/status text to include the new toggle when focus is Diagram.

## Testing

- Unit tests:
  - Model updates: setting/clearing notes marks the correct object refs updated in deltas.
  - Persistence: save → load roundtrips notes for nodes/participants.
- Render snapshot tests:
  - With notes disabled, rendering matches current baseline snapshots.
  - With notes enabled, boxes include a note line under the label and are clipped deterministically.

