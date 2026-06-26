DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/render/sequence/helpers.rs:826-848, src/format/mermaid/sequence.rs:713-722 | Slug: sequence-section-noncontiguous-render

# Sequence section frames span gap rows when message_ids are non-contiguous

## Finding

`section_row_ranges` computes block frame bounds using `min`/`max` over member message rows, so a section that skips intermediate messages draws a frame covering non-member rows. Export explicitly rejects the same non-contiguous membership.

## Violated Invariant Or Contract

Block/section frames must cover exactly the rows of member messages; export and render should agree on section validity.

## Oracle

`export_section_ranges` rejects non-contiguous indices with `InvalidBlockMembership` (`713-722`). `section_row_ranges` uses min/max without contiguity check (`826-848`).

## Counterexample

Three messages at rows 0,1,2. Alt section `message_ids = [m1, m3]` only. Export fails with "not contiguous". Render succeeds: `start_row=0`, `end_row=2`, framing row 1 (message m2) although m2 ∉ section.

## Why It Might Matter

Ops- or model-built ASTs with non-contiguous block membership render misleading frames; agents/users see incorrect visual grouping.

## Proof

Cross-entry mismatch: export validates contiguity; render uses `min_row..max_row` span. `SequenceSection::new` accepts any `message_ids` without model validation.

## Counterevidence Checked

Mermaid parse appends messages sequentially into sections (contiguous at parse time). Bug appears for programmatic AST mutation, not parse-only diagrams.

## Suggested Next Step

Align render with export: reject non-contiguous sections or draw per-message frames instead of min/max span.

## Status Notes

2026-06-26: Marked fixed after static validation. `section_row_ranges` now sorts and deduplicates member rows, rejects missing message ids, and returns `InvalidBlockMembership` when adjacent rows are not contiguous. Render now rejects the same non-contiguous section membership the exporter rejects, so it no longer draws a misleading min/max frame over skipped messages.

DEVANA-KEY: src/render/sequence/helpers.rs:826-848 | P2 | sequence-section-noncontiguous-render
DEVANA-SUMMARY: fixed P2 high src/render/sequence/helpers.rs:826-848 - Section frames use min/max row span, drawing over messages not in the section while export rejects the same layout.
