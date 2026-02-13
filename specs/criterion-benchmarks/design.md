# Design — 25-criterion-benchmarks

## Overview

This spec adds a Criterion benchmark suite that is intentionally structured to remain useful across refactors:

- Benchmarks target **stable entrypoints** (layout, routing, renderers, ops apply, store save) instead of private helper functions that may be moved/deduplicated.
- Benchmark **IDs remain stable**; when implementations change, we update the wiring, not the benchmark identity.
- Bench fixtures are **deterministic** and parameterized by explicit sizes so results can be compared across time.

## Benchmark identity + stability

### Naming scheme

Use a consistent scheme so refactors don’t create “new” benchmarks:

- Group: `flow.layout`, `flow.route`, `seq.layout`, `render.sequence`, `render.flow`, `ops.apply`, `store.save_session`, `scenario.persist_edit`
- Case id suffix: `small`, `medium`, `large`, plus optional shape modifiers (e.g. `dense`, `maze`, `long_labels`)

Example stable names:
- `flow.route/medium_dense`
- `render.flow/large_long_labels`
- `scenario.persist_edit/session_25_touch_1`

### Refactor resilience (dedup and code motion)

To account for upcoming dedup of render helpers and other code motion:

- Bench targets should call *public* or otherwise stable functions:
  - `layout_flowchart`, `route_flowchart_edges_orthogonal`
  - `layout_sequence`
  - `render_sequence_unicode`, `render_flowchart_unicode` (or `render_diagram_unicode` where appropriate)
  - `apply_ops`
  - `SessionFolder::save_session`
- If a refactor changes which implementation is “canonical” (e.g. shared renderer helper), the benchmark should continue to measure the same **workload** by staying attached to the entrypoint(s).
- If a refactor requires changing call sites, update the benchmarks without changing benchmark IDs.

## Fixture design (deterministic generators)

All benchmarks use deterministic fixtures built from explicit parameters (no RNG unless a fixed seed is encoded into the case id).

### Flowchart fixtures

Generate layered DAGs with configurable density and routing stress:

- Parameters:
  - `layers`, `nodes_per_layer`
  - `fanout` (edges from each node to next layer)
  - `cross_edges` (additional edges skipping layers to stress routing)
  - `label_len` (short vs long labels)
- Derived work units:
  - nodes: `N`
  - edges: `E`

Include at least:
- `small`: ~50–150 edges
- `medium`: ~500–2k edges
- `large`: ~5k–20k edges (tune to keep runtime reasonable)

### Sequence fixtures

Generate:
- participants `P`
- messages `M` with a deterministic order_key distribution
- text variants: short vs long message text

Derived work units:
- participants: `P`
- messages: `M`

### Session fixtures (store + scenario)

Generate a `Session` containing a mix of sequence + flow diagrams and (optionally) walkthroughs/xrefs:
- session sizes: `session_small`, `session_medium`, `session_large`
- ensure IDs are stable and deterministic

For persistence benchmarks, provide two variants:
- **compute-only**: run export/layout/render in memory (no filesystem)
- **filesystem**: call `SessionFolder::save_session` into a per-run temp directory

## Bench suite structure

### Files

- `benches/flow.rs`
  - `flow.layout/*` → `layout_flowchart`
  - `flow.route/*` → `route_flowchart_edges_orthogonal` (requires layout)
- `benches/seq.rs`
  - `seq.layout/*` → `layout_sequence`
- `benches/render.rs`
  - `render.sequence/*` → `render_sequence_unicode` (requires layout)
  - `render.flow/*` → `render_flowchart_unicode` (requires layout + routing)
- `benches/ops.rs`
  - `ops.apply/*` → `apply_ops` with controlled op batches
- `benches/store.rs`
  - `store.save_session/*` → `SessionFolder::save_session`
  - plus compute-only export/layout/render loop (no I/O)
- `benches/scenario.rs`
  - `scenario.persist_edit/*` → simulate “apply a small change then persist” to track future incremental-save refactors

### Noise control

- Prefer CPU-bound benches for algorithmic tracking; keep I/O benches separate and clearly labeled.
- Use `black_box` and/or a checksum of outputs to prevent optimizations.
- Use Criterion throughput where meaningful:
  - routing: `Throughput::Elements(edges)`
  - rendering: `Throughput::Bytes(ascii.len() as u64)` (or elements = objects/messages)
  - ops: `Throughput::Elements(ops.len() as u64)`

## Baseline workflow (tracking refactors)

Criterion supports local baselines; the workflow should be documented and stable:

1. Run benchmarks on `main` and save a baseline (name includes date or commit).
2. Run the same benchmarks on the refactor branch, comparing to that baseline.
3. Keep benchmark IDs unchanged so comparisons remain meaningful.

Optional: add a small helper script in `scripts/` to standardize invocation (feature flags, bench filters, baseline naming).

