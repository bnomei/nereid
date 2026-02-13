# Requirements — DRAFT-38-sequence-blocks

This spec adds **sequence diagram blocks** (Mermaid combined fragments) as first-class, stable AST objects for agent-friendly reasoning and deterministic rendering.

Normative protocol reference: `docs/protocol-01.md`

Checklist mapping: `docs/mm-as.md` item **#38**.

## Scope

Supported Mermaid-ish subset additions (inside `sequenceDiagram`):
- `alt <header>` … `else <header>` … `end`
- `opt <header>` … `end`
- `loop <header>` … `end`
- `par <header>` … `and <header>` … `end`

Notes:
- `<header>` is optional; if present it is the rest of the trimmed line after the keyword.
- Blocks may be nested up to a fixed maximum depth (see requirements).

## Non-goals

- Activation bars (`activate`/`deactivate`).
- Mermaid notes (`Note …`) (handled by a separate spec).
- `critical`, `break`, `rect` fragments.
- Rewriting the timeline/layout model for messages (blocks render as decorations around message rows).
- Adding new MCP mutation primitives for blocks (can be layered later; this spec focuses on parse/export/render + stable refs).

## Requirements (EARS)

### Parse + export

- WHEN `parse_sequence_diagram` encounters a supported block start keyword (`alt`, `opt`, `loop`, `par`), THE SYSTEM SHALL open a new block with a stable `ObjectId`.
- WHEN `parse_sequence_diagram` encounters a supported block section keyword (`else`, `and`) inside a matching open block, THE SYSTEM SHALL open a new section within the current block with a stable `ObjectId`.
- WHEN `parse_sequence_diagram` encounters `end`, THE SYSTEM SHALL close the most recently opened block; IF no block is currently open, THEN THE SYSTEM SHALL return an actionable parse error with the line number.
- WHEN unsupported sequence syntax is encountered, THE SYSTEM SHALL reject it with an actionable error containing the line number and the unsupported line.

- WHEN exporting a `SequenceAst` containing blocks, THE SYSTEM SHALL emit canonical `.mmd` that is deterministic for identical ASTs:
  - participants remain in stable order,
  - messages remain in `(order_key, message_id)` order,
  - blocks/sections emit in a stable order derived from their declaration order and membership.

### Structural validity

- WHEN a block (or any of its sections) contains no messages (including via nested blocks), THE SYSTEM SHALL reject the diagram as unsupported with an actionable error (empty sections are not supported in this baseline).
- WHEN blocks are nested deeper than a fixed constant `MAX_BLOCK_NEST_DEPTH`, THE SYSTEM SHALL reject parsing with an actionable error.

### Rendering

- WHEN rendering a sequence diagram containing blocks, THE SYSTEM SHALL render each block as a deterministic Unicode decoration around the message rows it covers, without overwriting message arrows/text.
- WHEN rendering a block with multiple sections, THE SYSTEM SHALL render deterministic section separators and render the section header keywords (`else`/`and`) in a stable position.
- WHEN rendering nested blocks, THE SYSTEM SHALL render nested decorations as inset frames (deterministic inset per depth) up to `MAX_BLOCK_NEST_DEPTH`.

### Stable references + highlighting

- THE SYSTEM SHALL provide stable `ObjectRef`s for block objects and make them addressable by:
  - `d:<diagram_id>/seq/block/<block_id>`
  - `d:<diagram_id>/seq/section/<section_id>`
- WHEN producing annotated render output, THE SYSTEM SHALL include block/section decoration cells in the highlight spans for the corresponding object refs and SHALL clamp spans to the returned text.

