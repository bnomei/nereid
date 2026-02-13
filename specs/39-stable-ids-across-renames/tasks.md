# Tasks — DRAFT-39-stable-ids-across-renames

Meta:
- Spec: DRAFT-39-stable-ids-across-renames — Persist stable IDs across Mermaid-visible renames
- Depends on: spec:05-mermaid-format, spec:09-session-store, spec:04-ops-delta, spec:19-mcp-protocol-surface
- Global scope:
  - src/store/session_folder.rs
  - src/format/mermaid/flowchart.rs
  - src/ops/mod.rs
  - src/mcp/types.rs
  - src/mcp/server.rs

## In Progress

- [ ] T001: Store — persist `stable_id_map` in diagram sidecars (owner: worker:019c4bc2-d400-7822-b4d9-c284e4a4a682) (scope: src/store/session_folder.rs) (depends: -)
  - Started_at: 2026-02-11T08:12:58Z
  - Context: `DiagramMeta.stable_id_map` exists but `save_session()` currently writes it as `{}`; load-time reconciliation needs it.
  - DoD:
    - `SessionFolder::save_session` computes and writes `stable_id_map`:
      - sequence: `by_name[participant.mermaid_name] = participant_id`
      - flowchart: `by_mermaid_id[node.mermaid_id_or_fallback] = node_id`
    - Mapping is deterministic and excludes empty keys.
  - Validation:
    - `cargo test`
  - Escalate-if:
    - You need to change the on-disk schema beyond populating existing `stable_id_map` fields.

- [ ] T003: Mermaid flowchart — preserve/use `FlowNode.mermaid_id` in parse/export (owner: worker:019c4bc2-d70f-7051-8cdb-63aca853fb09) (scope: src/format/mermaid/flowchart.rs) (depends: -)
  - Started_at: 2026-02-11T08:12:58Z
  - Context: flow export currently derives Mermaid ids from `node_id` (`n:<id>`), which prevents “rename Mermaid id without changing stable id”.
  - DoD:
    - Parser sets `FlowNode.mermaid_id = Some(<parsed mermaid id>)` for all nodes.
    - Export references nodes by `FlowNode.mermaid_id` (fallback to legacy derive-from-`node_id` when missing).
    - Export remains deterministic; existing tests updated or extended.
  - Validation:
    - `cargo test`
  - Escalate-if:
    - You need to change `FlowchartAst`/`FlowNode` public APIs (prefer staying within current model surface).

## Blocked

- (none)

## Todo

- [ ] T002: Store — remap participants/nodes on load using `stable_id_map` (owner: unassigned) (scope: src/store/session_folder.rs) (depends: T001)
  - Context: `load_session()` currently reconciles edges/messages but not their endpoints; after renames, IDs drift.
  - DoD:
    - Add `reconcile_sequence_participants(ast, sidecar)` and call it before `reconcile_sequence_messages`.
    - Add `reconcile_flowchart_nodes(ast, sidecar)` and call it before `reconcile_flowchart_edges`.
    - Remap rewrites:
      - sequence: participant map keys + all message from/to participant ids
      - flowchart: node map keys + all edge from/to node ids (+ node_groups map if populated)
    - Best-effort behavior when mappings are missing or collide (no panics; load succeeds).
  - Validation:
    - `cargo test`
  - Escalate-if:
    - You discover additional model indices that must be remapped outside this file (split into a new task with tighter scope).

- [ ] T004: Ops — add flow-node Mermaid id rename op (owner: unassigned) (scope: src/ops/mod.rs) (depends: T003)
  - Context: renames should be structured ops; stable node ids must not change.
  - DoD:
    - Add an op variant (internal) that sets/clears a node’s Mermaid id without changing `node_id`.
    - Validate identifier validity + uniqueness within the flowchart AST.
    - Delta marks the node as `updated`.
  - Validation:
    - `cargo test`
  - Escalate-if:
    - You need to introduce new shared validation utilities outside this file (propose where they should live first).

- [ ] T005: MCP — expose flow-node Mermaid id rename op (owner: unassigned) (scope: src/mcp/types.rs, src/mcp/server.rs) (depends: T004)
  - Context: MCP currently cannot express flow node id renames; required to avoid manual `.mmd` edits.
  - DoD:
    - Extend `McpOp` with the new flow op variant.
    - Map MCP op ↔ internal op and handle it in `diagram.apply_ops` + `diagram.propose_ops`.
    - Add/extend MCP server tests for the new op (happy path + invalid params).
  - Validation:
    - `cargo test`
  - Escalate-if:
    - MCP schema changes require coordinating with external clients (capture the breaking change and stop).

- [ ] T006: Store tests — regression: renames survive save/load without breaking refs (owner: unassigned) (scope: src/store/session_folder.rs) (depends: T001, T002, T003)
  - Context: the bug manifests only after save → reload, because `.mmd` re-parsing reallocates ids.
  - DoD:
    - Add a test that constructs a session where:
      - sequence participant stable id != current mermaid name (e.g. id `p:alice`, name `Alicia`)
      - flow node stable id != current mermaid id (e.g. id `n:authorize`, mermaid id `authz`)
      - xrefs target the stable ids
    - Save session, load session, and assert:
      - loaded session equals original (or equivalently: stable ids preserved + xrefs not dangling)
      - sidecar `stable_id_map` includes the new names/ids mapping to the stable ids
  - Validation:
    - `cargo test`
  - Escalate-if:
    - You cannot express the regression without adding new public helpers; propose the smallest helper signature.

## Done

- (none)
