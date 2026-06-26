DEVANA-FINDING: v1
DEVANA-STATE: invalid
Priority: P2 | Confidence: high | Security-sensitive: no | Status: invalid
Location: src/format/mermaid/flowchart.rs:547-548,822-892 | Slug: linkstyle-edge-index-roundtrip

# linkStyle edge index differs between parse order and export sort order

## Finding

`parse_flowchart` records edges in file declaration order for `linkStyle N` indexing, but `export_flowchart` sorts edges by `(from_node_id, to_node_id, edge_id)` before assigning indices. Round-tripping Mermaid with `linkStyle` can attach styles to the wrong edge when declaration order differs from sort order.

## Violated Invariant Or Contract

`linkStyle N` must refer to the same semantic edge across parse → model → export → re-parse. Module docs state linkStyle is "preserved on export."

## Oracle

`parse_flowchart` doc comment at line 540; `semantic_roundtrip_parse_export_parse` tests in the same file.

## Counterexample

```mermaid
flowchart
Z --> A
A --> B
linkStyle 0 stroke:#ff0000;
```

Parse: index 0 → `Z-->A` (first declared). Export sorts by `from`: `A-->B` becomes index 0. Re-parse attaches red stroke to `A-->B` instead of `Z-->A`.

## Why It Might Matter

Agents using `diagram.create_from_mermaid` / export lose or misassign edge styles on flowcharts where edges are not already in sorted `from` order.

## Proof

**Contract mismatch:** Parse uses `parsed_edges` append order (`547-548`, applied at `709-727`). Export uses sorted `edges` with `enumerate()` at `822-830`, `877-892`.

**Counterexample value:** Two edges where first declared edge has lexicographically larger `from` id than second.

## Counterevidence Checked

Existing roundtrip tests use edges whose parse order matches export sort order (e.g. `A-->B` before `A-->C`). No test covers `linkStyle` with reordering.

## Suggested Next Step

Export `linkStyle` indices using the same edge ordering as parse (declaration order stored in model), or store style keyed by edge id rather than positional index.

## Status Notes

2026-06-26: Marked invalid after static validation. `parse_flowchart` attaches `linkStyle` to the semantic edge, and `export_flowchart` emits each styled link index from the same sorted edge order it writes to the Mermaid output. The exact `Z --> A`, `A --> B`, `linkStyle 0` counterexample re-exports with the style index pointing at the emitted `Z --> A` edge, and the existing regression source covers that case. No code change was needed for this finding.

DEVANA-KEY: src/format/mermaid/flowchart.rs:547-548,822-892 | P2 | linkstyle-edge-index-roundtrip
DEVANA-SUMMARY: invalid P2 high src/format/mermaid/flowchart.rs:547-548,822-892 - linkStyle indices are assigned in declaration order on parse but sorted from-order on export, corrupting styles on round-trip.
