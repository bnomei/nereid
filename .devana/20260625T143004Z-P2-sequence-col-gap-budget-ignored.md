DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: open
Location: src/render/sequence.rs:30,148 | Slug: sequence-col-gap-budget-ignored

# Sequence renderer ignores layout column-gap spacing budget

## Finding

`layout_sequence` computes `col_gap_extra_spacing_by_col` to widen column gaps when participant labels or long inter-column message spans need more room. `render_sequence_unicode` uses a fixed `COL_GAP = 8` and never reads the layout budget, so rendered columns can overlap or truncate labels the layout already budgeted for.

## Violated Invariant Or Contract

Layout spacing budget is the coordinate oracle for render. Row budget (`row_extra_spacing_by_row`) and self-loop stub lengths are consumed by render; column gap budget should be too.

## Oracle

`src/layout/sequence.rs:308-358` builds `col_gap_extra_spacing_by_col`; layout test at line 651 asserts it is non-empty for long-span fixtures. Render places columns at `cursor_x = box_x1 + 1 + COL_GAP` (`src/render/sequence.rs:148`).

## Counterexample

Three participants with a long `Alice->>Carol` message label exceeding baseline span capacity (same fixture as layout spacing test). Layout sets non-empty `col_gap_extra_spacing_by_col`; render advances columns by fixed 8 cells regardless.

## Why It Might Matter

Unicode sequence diagrams show overlapping lifelines or clipped message labels for realistic diagrams, despite layout computing correct spacing.

## Proof

**Dataflow trace:** `build_sequence_spacing_budget` → `col_gap_extra_spacing_by_col` → (not read) → `render_sequence_unicode` uses `COL_GAP` constant only.

**Cross-entry mismatch:** `row_extra_spacing_by_row` and `self_loop_stub_len_by_message_id` are consumed in render helpers; `col_gap_extra_spacing_by_col` appears only under `src/layout/`.

## Counterevidence Checked

Grep for `col_gap_extra` under `src/render/` returns no matches. Layout and render share `SequenceLayout` struct but render ignores the column-gap field.

## Suggested Next Step

Apply `layout.spacing_budget().col_gap_extra_spacing_by_col()` when computing participant `cursor_x` in `render_sequence_unicode`.

DEVANA-KEY: src/render/sequence.rs:30,148 | P2 | sequence-col-gap-budget-ignored
DEVANA-SUMMARY: P2 high src/render/sequence.rs:30,148 - Sequence render uses fixed COL_GAP and ignores layout's col_gap_extra_spacing_by_col, breaking label spacing for wide diagrams.