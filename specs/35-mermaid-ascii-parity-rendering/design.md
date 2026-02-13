# Design — DRAFT-35-mermaid-ascii-parity-rendering

## Overview

This spec adds **render-time decorations** (arrowheads, labels, rounded corners) and a small **sequence parse/export extension** (participant aliases), targeting better “discussion-quality” diagrams without expanding the Mermaid subset broadly.

Key design constraints:
- Keep **deterministic output** and stable highlight spans.
- Keep changes localized to `src/format/mermaid/`, `src/model/`, and `src/render/`.

## Flow: arrowheads (#22)

### Goal

Visually communicate direction on flow edges using Unicode arrowheads.

### Placement strategy (baseline)

1) Compute the rendered connector polyline in canvas coordinates:
   - Routed edges already have a `Vec<GridPoint>` route; convert it to a list of canvas points using the same adapter logic as drawing.
   - Non-routed edges use the existing L-shaped connector points (`from.mid_y` to `to.mid_y` via a bend).

2) Determine the direction of the **final segment**:
   - If last segment is horizontal: use `▶` (right) or `◀` (left).
   - If last segment is vertical: use `▼` (down) or `▲` (up).

3) Place the arrowhead on the final segment **without breaking node-border junctions**:
   - Prefer placing the arrowhead **two cells before** the target-side box border/junction cell on horizontal segments (or two cells before the target y on vertical segments).
   - If insufficient room (short segment), fall back to one cell before; if still impossible, omit arrowhead for that edge (deterministic).

4) Draw order:
   - Draw connector segments first (preserves box-junction merging).
   - Then place the arrowhead (non-box char overwrite).

### Highlight spans

Include the arrowhead cell in the edge’s highlight spans in annotated rendering.

## Flow: edge labels (#14/#23)

### Goal

Render `FlowEdge.label` on the edge path deterministically without overlapping nodes.

### Placement strategy (baseline)

1) Prefer a **horizontal segment** of the connector path with maximal length.
2) Reserve at least 1 cell at each end of the chosen segment for connector continuity and to avoid breaking tees/junctions.
3) Clip label to available width using existing `truncate_with_ellipsis`.
4) Write the label centered on that segment (stable choice; ties resolved deterministically by earliest segment in traversal order).

### Draw order

- Draw connector segments
- Write edge label text
- Place arrowhead (so arrowhead wins collisions)

### Highlight spans

The edge’s highlight spans should include:
- connector spans (existing behavior)
- label span (new)
- arrowhead span (new)

## Flow: rounded corners for `round` nodes (#20 limited)

### Goal

Render the existing model `shape == "round"` as a rounded-corner rectangle (no other shapes).

### Implementation

- Continue drawing the node’s border with the existing box-drawing lines.
- Overwrite the four corner cells with rounded corner glyphs:
  - top-left `╭`, top-right `╮`, bottom-left `╰`, bottom-right `╯`
- Do not change layout sizing rules (still based on label width).

Non-goal: render diamonds; `shape == "diamond"` continues to use the baseline box.

## Sequence: participant aliases (#35)

### Goal

Support Mermaid subset:
- `participant <alias> as <display>`

### Model change

Extend `SequenceParticipant` to represent both:
- **identifier**: the Mermaid token used in message lines (`alias`)
- **display label**: the text shown in the participant box (defaults to alias)

Suggested API:
- `ident(&self) -> &str` (current `mermaid_name` semantics)
- `label(&self) -> &str` (display; default = ident)

### Parse rules (subset)

- Accept:
  - `participant <ident>`
  - `participant <ident> as <display>`
- Require `<ident>` to satisfy existing `validate_mermaid_ident`.
- For now, restrict `<display>` to a single non-whitespace token (keeps parser std-only and mirrors current ident validation posture).

Conflict handling (deterministic):
- If the same `<ident>` is declared multiple times:
  - If display label matches: accept (idempotent).
  - If display label conflicts: reject with an actionable parse error.

### Export rules

- Participants are emitted in stable `ObjectId` order.
- For each participant:
  - If `label == ident`: emit `participant <ident>`
  - Else: emit `participant <ident> as <label>`
- Messages emit `<from_ident><arrow><to_ident>: <text>`

## Sequence: dotted return stroke rendering (#32)

### Goal

Render return messages (`-->>`) with a dotted/dashed stroke in Unicode mode.

### Approach (minimal refactor)

Avoid refactoring `Canvas` line-merging:
- Render the return message arrow using the existing solid-line algorithm (so junctions/tees remain correct).
- Post-process the rendered text for that message row/span only:
  - Replace `─` with `┈` in the horizontal run between the sender-side junction and the arrowhead (excluding the junction cell and the arrowhead cell).

This keeps:
- deterministic geometry
- highlight spans unchanged (cell coordinates stable)
- no changes to `Canvas` internals

## Testing strategy

- Add snapshot-style rendering tests that lock in:
  - flow: arrowheads + labels + rounded corners
  - sequence: aliases affect rendered participant labels and canonical export
  - sequence: return arrows use dashed stroke (`┈`) in the expected span

