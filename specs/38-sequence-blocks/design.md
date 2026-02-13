# Design — DRAFT-38-sequence-blocks

## Overview

Protocol 01 prioritizes blocks (`alt/else`, `opt`, `loop`, `par`) because they encode software discussion semantics:
- guards/branching
- optional paths
- retries/polling
- concurrency

This spec implements blocks as **stable AST objects** and renders them as **decorations** around message rows (no new timeline layout model).

## Model

Extend `SequenceAst` with:
- `blocks: Vec<SequenceBlock>`

Types (sketch):
- `SequenceBlock { block_id, kind, header, sections: Vec<SequenceSection>, blocks: Vec<SequenceBlock> }`
  - `blocks` are nested blocks (tree).
- `SequenceSection { section_id, kind, header, message_ids: Vec<ObjectId> }`
  - `kind` is `Main|Else|And` (or the keyword used).

Stable IDs:
- `block_id`: `ObjectId` like `b:0001` allocated in parse order.
- `section_id`: `ObjectId` like `sec:0001:00` (block index + section index), allocated deterministically.

Block membership:
- As messages are parsed, the current open *section* receives the new `message_id`.
- Nested blocks are stored within the parent block; their messages also count toward the parent’s “contains at least one message” validity check.

## Parsing + export

### Parser strategy

Use a stack of open blocks; each open block tracks its “current section”.

Accepted lines (in addition to existing participants/messages):
- `alt [header...]`
- `else [header...]`
- `opt [header...]`
- `loop [header...]`
- `par [header...]`
- `and [header...]`
- `end`

Validation:
- `else` is only valid inside an `alt`.
- `and` is only valid inside a `par`.
- `end` closes the most recent open block.
- Enforce `MAX_BLOCK_NEST_DEPTH` at parse time.
- Enforce “each section has >= 1 message (direct or nested)” after parse completes.

### Export strategy (canonical)

Emit:
1) `sequenceDiagram`
2) participants (existing stable order)
3) a merged stream of:
   - block open lines,
   - message lines,
   - section split lines (`else`/`and`),
   - block close lines (`end`)

Because blocks are stored in the AST, the exporter must not rely on textual round-tripping; it emits a deterministic canonical structure from IDs + stored membership.

## Rendering

### Key insight: use gap rows (avoid overwriting message rows)

Sequence messages are rendered at `y = message_top_y + row * ROW_SPACING` (ROW_SPACING is 2), leaving a deterministic **gap row** between messages.

Render all block borders and section separators on **gap rows** only:
- Top border: `message_y(start_row) - 1`
- Bottom border: `message_y(end_row) + 1`
- Section separator (e.g. `else`): `message_y(first_row_of_section) - 1`

This keeps message arrows/text intact while still drawing visible frames.

### Horizontal span

Use the full rendered canvas width (`x=0..=width-1`) for the outermost block frame.

Nested blocks inset deterministically:
- left/right inset per depth: `depth * 2` cells
- section labels start at `x = left + 2` (after the corner)

### Highlight spans

For each block/section, compute spans that cover:
- left + right vertical borders across the covered y-range,
- top/bottom borders,
- header label span on the top border,
- section separator border + header label spans.

Insert these spans into the `HighlightIndex` under the appropriate `ObjectRef`.

## Testing

- Parse/export tests:
  - `alt/else`, `opt`, `loop`, `par/and` canonical export contains expected keywords.
  - parse → export → parse preserves block/section structure and message membership.
- Render snapshot tests:
  - block decorations do not overwrite message label lines.
  - nested blocks render as inset frames deterministically.

