# Design — 23-diagram-meta-roundtrip

## Context (extracted from `docs/protocol-01.md` §7)

- Mermaid `.mmd` is an interchange/export format generated from the AST.
- Stable internal IDs must survive save/load even though Mermaid cannot carry IDs for all objects (especially sequence messages and flow edges).
- The `.meta.json` sidecar is the carrier for stable-ID reconciliation hints and any non-roundtrippable fields.

## Approach

1. **Wire sidecar into session save/load**
   - On save, write `diagrams/<stem>.meta.json` for each diagram (atomic write, session-relative paths).
   - On load, if the sidecar exists, load it and reconcile the freshly-parsed AST.

2. **Persist what Mermaid can’t represent**
   - Sequence: a deterministic list of message fingerprints → stable `message_id`.
   - Flowchart: a deterministic list of edge fingerprints → stable `edge_id`, plus non-Mermaid fields (e.g. `style`).

3. **Reconciliation algorithm (best-effort, deterministic)**
   - Build a multimap from fingerprint → stable-id entries from the sidecar (preserving deterministic order).
   - For each parsed message/edge, compute its fingerprint and claim the next available stable-id entry.
   - Unmatched parsed objects keep their newly-generated IDs.
   - Unclaimed sidecar entries are ignored (represent deleted objects).

## Scope / ownership

- Primary: `src/store/session_folder.rs`
- Secondary (only if needed): diagram meta structs/schema in the store layer

Non-goals:
- UI state persistence
- undo/redo/oplog

