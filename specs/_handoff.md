# Program handoff

Last updated: 2026-02-11T08:13:53Z

## Current focus
- `39-stable-ids-across-renames` — active implementation wave (`T001`, `T003`)
- `34-perf-flow-layout` — active perf wave (`T001`)
- `31-perf-flow-routing` — blocked follow-ups (`T005`, `T007`); next viable: `T008`

## Reservations (in progress scopes)
- `39-stable-ids-across-renames/T001` (owner: worker:019c4bc2-d400-7822-b4d9-c284e4a4a682)
  - Scope:
    - `src/store/session_folder.rs`
  - Validation:
    - `cargo test`
- `39-stable-ids-across-renames/T003` (owner: worker:019c4bc2-d70f-7051-8cdb-63aca853fb09)
  - Scope:
    - `src/format/mermaid/flowchart.rs`
  - Validation:
    - `cargo test`
- `34-perf-flow-layout/T001` (owner: worker:019c4bc2-d9a4-75a3-9c16-f49040fa3ac7)
  - Scope:
    - `src/layout/flowchart.rs`
  - Validation:
    - `cargo test`
    - `./scripts/bench-criterion compare --bench flow --baseline perf-layout-pre -- '^flow\\.layout/large_long_labels$'`

## In progress tasks
- `39-stable-ids-across-renames/T001`: dispatched; implementing sidecar `stable_id_map` save path.
- `39-stable-ids-across-renames/T003`: dispatched; implementing `FlowNode.mermaid_id` parse/export roundtrip.
- `34-perf-flow-layout/T001`: dispatched; profiling/editing to reduce dominant layout `String::clone` overhead.

## Blockers
- `31-perf-flow-routing/T005` — did not greenlight; likely requires API change (or accept that `flow.route/*` includes output-map overhead)
- `31-perf-flow-routing/T007` — attempted; did not greenlight reliably across focused compares; change discarded

## Next ready tasks
- `39-stable-ids-across-renames/T002` (after `T001`)
- `39-stable-ids-across-renames/T004` (after `T003`)
- `32-perf-flow-render-large-labels/T006`

## Notes
- Repo note: `specs/` is currently ignored by git; ledgers/handoff won’t show in `git status`.
- Keep dispatch rolling: when one worker finishes, immediately reserve and dispatch the next disjoint-scope ready task.
