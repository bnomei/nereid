# Requirements — DRAFT-37-inline-notes

This spec adds **inline notes** for diagram objects as a collaboration affordance: short invariants/data-shape hints attached to a **flow node** or **sequence participant**.

The user explicitly wants notes rendered **inside the object box**, on the line under the main label, and a UI toggle (`n`) to show/hide notes in the diagram view.

Normative protocol reference: `docs/protocol-01.md`

Checklist mapping: `docs/mm-as.md` item **#37** (implemented with a simplified in-app note model; not Mermaid `Note left of ...` syntax).

## Non-goals

- Mermaid `Note left/right of …` syntax parsing/export.
- Notes attached to sequence messages or flow edges.
- Multi-line notes (single line only; clip with ellipsis).
- Persisting the “show notes” toggle to disk (view-only preference).

## Requirements (EARS)

### Model + persistence

- WHEN a flow node has a note, THE SYSTEM SHALL store the note text in the in-memory AST/model associated with that node.
- WHEN a sequence participant has a note, THE SYSTEM SHALL store the note text in the in-memory AST/model associated with that participant.
- WHEN a session is saved and re-loaded, THE SYSTEM SHALL preserve node/participant notes (notes are not represented in canonical `.mmd`).

### Rendering

- WHEN notes are enabled in the diagram view, THE SYSTEM SHALL render a node/participant note inside the object’s box on the line directly under the main label.
- WHEN notes are disabled in the diagram view, THE SYSTEM SHALL render diagrams using the existing baseline box height and SHALL NOT render note text.
- WHEN a note does not fit within the box inner width, THE SYSTEM SHALL clip deterministically (e.g. ellipsis).

### UI toggle

- WHEN the TUI focus is the diagram view AND search mode is inactive, AND the user presses `n`, THE SYSTEM SHALL toggle “show notes” on/off and re-render the diagram accordingly.
- WHEN search mode is active (editing or results), `n` SHALL retain its existing meaning for search navigation/typing.

### Agent collaboration (MCP/ops)

- WHEN an agent applies an operation to set or clear a node/participant note, THE SYSTEM SHALL update the model and return a delta that marks the target object as updated.
- WHEN `diagram.get_ast` is called, THE SYSTEM SHALL include note fields for nodes/participants so an agent can reason about them without parsing rendered text.

