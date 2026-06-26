DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
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

## Status Notes

2026-06-26: Still open after static validation, with a partial fix in place. `actor` now round-trips: the parser accepts `actor <name>` and sets `participant.role()` to `Some("actor")`. The public model still accepts arbitrary role strings and the exporter writes any role verbatim; for example, a model-built participant with role `boundary` exports `boundary Alice`, but the parser only accepts `participant` and `actor`, so export -> parse can still fail for other role values.

2026-06-26: Marked fixed after teaching the sequence parser to accept two-token role declarations emitted by the exporter, while keeping known sequence control/directive keywords out of the custom-role path. Export now rejects reserved/unparseable role keywords instead of writing role lines the parser cannot round-trip. Regressions cover a model-built `boundary` role export -> parse round-trip, ensure unsupported `activate Alice` remains rejected instead of becoming a fake custom role, and verify a reserved `activate` role fails export.

DEVANA-KEY: src/format/mermaid/sequence.rs:847-856 | P2 | sequence-role-export-parse
DEVANA-SUMMARY: fixed P2 high src/format/mermaid/sequence.rs:847-856 - Participant roles exported as Mermaid role syntax cannot be parsed back into the AST.
