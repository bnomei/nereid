# Requirements — DRAFT-39-stable-ids-across-renames

This spec fixes **identity drift** when Mermaid-visible identifiers (sequence participant names; flowchart node ids) change over time.

Today, parsing effectively binds internal object IDs to Mermaid identifiers (e.g. `p:<name>`, `n:<mermaid_id>`). After a rename, the canonical `.mmd` output changes; on reload the parser allocates different IDs; existing `ObjectRef`s (xrefs, walkthrough refs, selection) can become dangling.

We **do not** need to support manual edits to `.mmd` files. Mermaid files are treated as canonical-generated exports; renames happen via in-app ops (TUI/MCP), and persistence must keep stable IDs intact across save/load.

## Non-goals

- Supporting manual `.mmd` edits / three-way merges / heuristic recovery for arbitrary file changes.
- Migrating to an all-new stable ID scheme (e.g. rewriting existing `p:*`/`n:*` IDs to numeric UUID-ish IDs).
- Exposing a “natural language rename” tool; this is structured ops only.

## Requirements (EARS)

### Stable identity

- WHEN a sequence participant’s Mermaid-visible name changes, THE SYSTEM SHALL keep the participant’s internal `ObjectId` stable.
- WHEN a flowchart node’s Mermaid-visible id changes, THE SYSTEM SHALL keep the node’s internal `ObjectId` stable.
- WHEN a session is saved and re-loaded, THE SYSTEM SHALL preserve `ObjectRef` identity for participants and flow nodes (refs that pointed to an object before save SHALL still resolve after load).

### Sidecar mapping (`stable_id_map`)

- WHEN saving a sequence diagram, THE SYSTEM SHALL persist a mapping from current participant Mermaid names to stable participant ids in the diagram sidecar (`.meta.json`) `stable_id_map.by_name`.
- WHEN saving a flowchart diagram, THE SYSTEM SHALL persist a mapping from current node Mermaid ids to stable node ids in the diagram sidecar (`.meta.json`) `stable_id_map.by_mermaid_id`.

### Load-time reconciliation

- WHEN loading a diagram with a non-empty `stable_id_map`, THE SYSTEM SHALL remap parsed participants/nodes to their persisted stable ids **before** reconciling sequence messages / flow edges.
- WHEN a mapping entry is missing (legacy sessions / partial sidecars), THE SYSTEM SHALL fall back to best-effort behavior (load succeeds; unmapped objects keep their parsed ids).

### Mermaid export behavior

- WHEN exporting a flowchart to canonical Mermaid, THE SYSTEM SHALL reference nodes by their stored Mermaid id field (not by deriving the Mermaid id from the stable internal node id).

### Ops surface (renames are structured)

- WHEN an agent/user renames a flowchart node’s Mermaid id via an operation, THE SYSTEM SHALL validate the new Mermaid id (identifier validity + uniqueness within the diagram) and then update the model without changing the node’s stable `ObjectId`.
- WHEN an agent/user renames a sequence participant via an operation, THE SYSTEM SHALL update the model without changing the participant’s stable `ObjectId`.

