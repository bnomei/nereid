DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: open
Location: src/format/mermaid/sequence.rs:847-856,365-400 | Slug: sequence-role-export-parse

# Sequence participant role export cannot be re-parsed

## Finding

`export_sequence_diagram` emits Mermaid role syntax (e.g. `actor Alice`) when `participant.role()` is set, but `parse_sequence_diagram` only accepts the `participant` keyword and never populates `role`.

## Violated Invariant Or Contract

Export → parse round-trip must preserve participant semantics including `SequenceParticipant.role` set via model/ops.

## Oracle

Model supports `set_role` (`seq_ast.rs`); ops preserve role (`ops/tests.rs`). Export branches on `role()` (`847-856`); parser rejects or mishandles non-`participant` declarations (`365-400`).

## Counterexample

Build AST: participant Alice with `role = Some("actor")`. Export emits `actor Alice\n`. `parse_sequence_diagram(&exported)` fails with `InvalidParticipantDecl` or parses without restoring `role()`.

## Why It Might Matter

MCP/TUI save-and-reload or mermaid round-trips silently drop participant roles, breaking diagrams that use Mermaid actor/boundary syntax.

## Proof

Contract mismatch: exporter writes role-prefixed lines; parser only handles `participant <name>` with no role field assignment.

## Counterevidence Checked

No `actor`/`boundary` parsing elsewhere in `format/mermaid/sequence.rs`. Parse-time diagrams never set role via Mermaid input in current parser.

## Suggested Next Step

Teach parser to accept Mermaid role keywords and set `participant.role()`, or export always as `participant` with a supported role annotation.

DEVANA-KEY: src/format/mermaid/sequence.rs:847-856 | P2 | sequence-role-export-parse
DEVANA-SUMMARY: P2 high src/format/mermaid/sequence.rs:847-856 - Participant roles exported as Mermaid role syntax cannot be parsed back into the AST.