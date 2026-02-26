# Render Quality V2 Fixture Corpus Manifest (v2)

## Purpose

This manifest defines the deterministic fixture corpus used by:

- Phase B baseline capture (`spec:41-render-determinism-baseline-b`, gate `G-B-BASE-003`).
- Phase C sequence-quality comparisons (`G-C-SEQ-001`, `G-C-SEQ-002`).
- Phase D flow-quality comparisons (`G-D-FLOW-002`).

## Determinism contract

- Fixture generation source: `benches/fixtures/mod.rs`.
- Bench mapping source: `benches/render.rs` and `benches/flow.rs`.
- All fixtures are generator-based and parameterized; no RNG/time input is used.
- Fixture IDs and parameter tuples in this file are normative for baseline capture.
- If any fixture ID or parameters change, this manifest version MUST be bumped and phase-B baseline MUST be recaptured.

## Canonical corpus for Render Quality V2

### Sequence fixtures

| Manifest fixture ID | Generator | Parameters | Bench IDs |
| --- | --- | --- | --- |
| `seq.small` | `fixtures::seq::fixture(fixtures::seq::Case::Small)` | `participants=8`, `messages=40`, `long_text=false` | `render.sequence/small` |
| `seq.small_long_text` | `fixtures::seq::fixture(fixtures::seq::Case::SmallLongText)` | `participants=8`, `messages=40`, `long_text=true` | `render.sequence/small_long_text` |

### Flow fixtures

| Manifest fixture ID | Generator | Parameters | Bench IDs |
| --- | --- | --- | --- |
| `flow.small` | `fixtures::flow::fixture(fixtures::flow::Case::Small)` | `layers=6`, `nodes_per_layer=10`, `fanout=2`, `cross_edges_per_node=0`, `label_len=12` | `render.flow/small`, `flow.route/small` |
| `flow.medium_dense` | `fixtures::flow::fixture(fixtures::flow::Case::MediumDense)` | `layers=12`, `nodes_per_layer=20`, `fanout=4`, `cross_edges_per_node=1`, `label_len=12` | `render.flow/medium_dense`, `flow.route/medium_dense` |
| `flow.dense_crossing` | `fixtures::flow::fixture(fixtures::flow::Case::DenseCrossing)` | `layers=14`, `nodes_per_layer=24`, `fanout=5`, `cross_edges_per_node=3`, `label_len=24` | `render.flow/dense_crossing`, `flow.route/dense_crossing` |
| `flow.large_long_labels` | `fixtures::flow::fixture(fixtures::flow::Case::LargeLongLabels)` | `layers=24`, `nodes_per_layer=35`, `fanout=4`, `cross_edges_per_node=2`, `label_len=64` | `render.flow/large_long_labels`, `flow.route/large_long_labels` |
| `flow.routing_stress` | `fixtures::flow::fixture(fixtures::flow::Case::RoutingStress)` | `layers=16`, `nodes_per_layer=30`, `fanout=3`, `cross_edges_per_node=4`, `label_len=12` | `render.flow/routing_stress`, `flow.route/routing_stress` |
| `flow.routing_stress_wide` | `fixtures::flow::fixture(fixtures::flow::Case::RoutingStressWide)` | `layers=18`, `nodes_per_layer=34`, `fanout=4`, `cross_edges_per_node=5`, `label_len=16` | `flow.route/routing_stress_wide` |

## Gate and phase usage mapping

| Program gate / phase | Uses baseline from this manifest | Required bench IDs |
| --- | --- | --- |
| `G-B-BASE-003` (phase B) | Capture baseline references | `render.sequence/*`, `render.flow/*`, `flow.route/*` |
| `G-C-SEQ-001` (phase C) | Compare sequence quality vs phase-B baseline | `render.sequence/*` |
| `G-C-SEQ-002` (phase C) | Recheck determinism while sequence quality changes land | Re-run determinism tests plus `render.sequence/*` compare evidence |
| `G-D-FLOW-002` (phase D) | Compare flow readability/routing quality vs phase-B baseline | `render.flow/*`, `flow.route/*` |

## Reproducibility notes

- Baseline capture steps and exact commands are defined in:
  - `specs/40-render-quality-v2-program/baseline-capture-protocol.md`
- Phase B is the canonical baseline producer; phases C and D are baseline consumers.
