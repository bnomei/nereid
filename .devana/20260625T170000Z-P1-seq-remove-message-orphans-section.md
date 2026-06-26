DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P1 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/ops/ops_impl.rs:198-209,73-97 | Slug: seq-remove-message-orphans-section

# RemoveMessage/RemoveParticipant orphan section message_ids and break Mermaid export

## Finding

`SeqOp::RemoveMessage` (`src/ops/ops_impl.rs:198-209`) deletes a message from
`SequenceAst::messages` but never removes that message id from any
`SequenceSection::message_ids` that referenced it. `SeqOp::RemoveParticipant`
(`:73-97`) cascade-deletes every message touching the participant with the same
omission. Block/section membership is stored independently of the message list,
and the ops layer touches neither `blocks()` nor `sections()` (confirmed: no
`blocks_mut`/`sections_mut`/`message_ids` mutation anywhere under `src/ops/`).
The result is a dangling section reference to a message that no longer exists.

## Violated Invariant Or Contract

Every `ObjectId` in `SequenceSection::message_ids` must refer to a message
present in `SequenceAst::messages`. The Mermaid exporter enforces exactly this
invariant and hard-errors when it is violated.

## Oracle

Exporter contract in `src/format/mermaid/sequence.rs:692-702`: a section
referencing a missing message id returns
`Err(MermaidSequenceExportError::InvalidBlockMembership { reason: "section ...
references missing message id ..." })`. (Removing a middle message of a 3-member
section instead trips the "not contiguous" branch at `:713-722`.) Persistence
does not heal this: `remap_sequence_block_message_ids`
(`src/store/session_folder/helpers.rs:468-487`) only remaps surviving ids and
leaves unknown ids untouched.

## Counterexample

1. Import a sequence diagram with a 2-message `alt` block via
   `diagram.create_from_mermaid`; the parser sets
   `section.message_ids = [m:0001, m:0002]`.
2. Apply `Op::Seq(SeqOp::RemoveMessage { message_id: m:0002 })`. `apply_ops`
   succeeds, bumps rev, `messages == [m:0001]`, but `section.message_ids` is
   still `[m:0001, m:0002]`.
3. Export to Mermaid → `Err(InvalidBlockMembership "... references missing
   message id m:0002")`. The diagram is now un-exportable.

`RemoveParticipant` reaches the same dangling state for every message it
cascade-removes.

## Why It Might Matter

A single legal edit op can leave the diagram un-exportable to Mermaid. Any path
that serializes the diagram to `.mmd` (save, MCP export query) then fails on a
diagram that the user successfully edited — a persisted-state inconsistency and
workflow breakage, not just a render glitch.

## Proof

Control-flow + contract trace:
- `src/ops/ops_impl.rs:198-209` mutates `messages` only (`retain`), no section update.
- `src/ops/ops_impl.rs:73-97` cascade-removes messages, no section update.
- `src/model/seq_ast.rs`: sections hold `message_ids` independently of `messages`.
- `src/format/mermaid/sequence.rs:692-702`: consumer errors on the dangling id.
- `src/store/session_folder/helpers.rs:468-487`: persist path preserves the stale id.

## Counterevidence Checked

Searched all of `src/ops`, `src/store`, `src/query` for any
retain/prune/reconcile of `section.message_ids` after message removal — none
exists. Distinct from the filed `sequence-section-noncontiguous-render`
(render-side frame spanning on non-contiguous ids) and
`diagram-apply-ops-stale-xref` (xref status, not section membership): this is an
ops-layer failure to update the dependent block/section collection, reproducible
with one `RemoveMessage`, and it produces a hard export error rather than a
mis-render.

## Suggested Next Step

After removing messages in `RemoveMessage`/`RemoveParticipant`, prune the removed
ids from every `section.message_ids` (and drop sections/blocks that become
empty), or run a normalization pass in `apply_ops` before returning.

## Status Notes

2026-06-26: Marked fixed after static validation. `SeqOp::RemoveMessage` and cascade `SeqOp::RemoveParticipant` now call `prune_messages_from_blocks` with the removed ids. The pruning pass removes those ids from every section and drops empty sections/blocks, so removed sequence messages no longer leave dangling `section.message_ids` that break Mermaid export.

DEVANA-KEY: src/ops/ops_impl.rs:198-209 | P1 | seq-remove-message-orphans-section
DEVANA-SUMMARY: fixed P1 high src/ops/ops_impl.rs:198-209 - Removing a sequence message/participant leaves a dangling section message_id, making the edited diagram fail Mermaid export.
