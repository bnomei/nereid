# Protocol 01: Sessions, Diagrams, Persistence, and MCP Collaboration

Status: Draft  
Date: 2026-02-06  

This document defines the **in-app protocol** and **MCP-facing surface** for an ASCII/Unicode Mermaid-like diagramming TUI where a human and an agent collaborate on **sequence diagrams** and **flowcharts**.

The primary goal is to make diagram reasoning/creation efficient for agents by keeping a **stable, queryable AST/graph** in the background so the agent does **not** have to re-draw or re-derive the diagram each turn.

---

## 1) Core principles

1. **AST is source of truth**
   - ASCII is a *projection* of the AST.
   - Mermaid `.mmd` is an *interchange/export* format generated from the AST (canonical formatting; may change on save).

2. **Stable references**
   - Every meaningful object (node/edge/message/participant/block) has a stable identifier addressable via an `ObjectRef`.
   - Stable IDs enable incremental edits and targeted queries.

3. **Small context by default**
   - Agent tooling defaults to **digests, deltas, and slices**.
   - Full AST/ASCII dumps require explicit requests.

4. **Deterministic, 100% Rust**
   - Parsing, layout, routing, and rendering are implemented in Rust.
   - No external layout engines (Graphviz/ELK) and no network calls.

5. **Walkthroughs are first-class knowledge artifacts**
   - A walkthrough is a **shareable**, **annotated**, **drill-down** map built from AST objects and typed queries.
   - It exists to help humans/agents start high-level, then progressively explore details without re-deriving context.

### 1.1 Locked decisions (as of 2026-02-06)

- Mermaid `.mmd` is canonical-generated (formatting may change).
- Flowcharts support modern `flowchart` only (not legacy `graph`).
- ASCII export happens on every save (export-on-save).
- XRefs are first-class in the UI (panel + jump list).
- Dangling XRefs are allowed but must be explicitly flagged and TODO-retrievable.
- Session persistence is folder-based; metadata uses relative paths so the session folder can be renamed/moved.

---

## 2) In-memory data model

### 2.1 Session

A `Session` is the top-level container the TUI runs against.

Minimum fields:
- `session_id`: stable ID (e.g. UUID)
- `diagrams`: map of `DiagramId -> Diagram`
- `walkthroughs`: map of `WalkthroughId -> Walkthrough`
- `xrefs`: cross-diagram links (see §4)
- `active_diagram_id`: optional default target for UI/agent actions
- `active_walkthrough_id`: optional default target for walkthrough operations
- `ui_state`: view state (selection, scroll, panes)
- `oplog`: applied operations (audit + undo/redo)

### 2.2 Diagram

A `Diagram` is a single, typed artifact.

Minimum fields:
- `diagram_id`: stable ID (UUID or stable slug)
- `name`: display name (human-friendly)
- `kind`: `Sequence` | `Flowchart`
- `ast`: `SequenceAst` | `FlowchartAst`
- `rev`: monotonic revision number
- `source`: optional persistence metadata (`mmd_path`, `meta_path`, last-import hash, etc.)
- `render_prefs`: unicode/ascii mode, spacing, padding (render-only; not part of semantic AST)

### 2.3 Sequence AST (conceptual)

The AST must support:
- deterministic rendering
- querying by object ref
- stable insertion/removal without re-numbering everything

Suggested structure:
- `Participant { participant_id, mermaid_name, role? }` (role examples: `actor`, `service`, `db`)
- `Message { message_id, from_participant_id, to_participant_id, kind, text, order_key }`
- `Note { note_id, scope, text }` (optional but high value for invariants/data shapes)

Recommended “software development discussion” priorities (highest value first):
- Participants/roles and simple grouping/boundaries (e.g. Client/API/DB) (UI concept; AST can keep roles/tags)
- Message kinds: sync vs async vs return, including self-messages
- Branching/guards: `alt/else`
- Optional paths: `opt`
- Loops: `loop` (retries/backoff, polling, batching)
- Concurrency: `par`
- Notes/annotations (pre/post-conditions, invariants, idempotency, data shapes)
- Stable “step references” via `ObjectRef` (UI may show a simple step index, but refs remain stable)

Lower priority (later):
- Activation bars
- Create/destroy
- `critical` / `break`

`order_key` should allow stable insertion (e.g. fractional indexing / order-maintenance) without reassigning all subsequent message IDs.

### 2.4 Flowchart AST (conceptual)

Suggested structure:
- `Node { node_id, mermaid_id?, label, shape }`
- `Edge { edge_id, from_node_id, to_node_id, label?, style? }`
- Optional later:
  - subgraphs
  - ports/anchors
  - orientation `LR/TD/RL/BT`

Scope note:
- Parse/render modern Mermaid `flowchart` only (not legacy `graph`).

### 2.5 Walkthrough (conceptual)

A `Walkthrough` is a narrative/teaching layer over one or more diagrams in a session.

Key properties:
- **Shareable**: stored in the session folder as a file-based artifact.
- **Interactive**: UI presents it as a clickable diagram with an inspector showing details.
- **Drill-down**: the top level is intentionally coarse; deeper nodes are added as needed.
- **Evidence-first**: each node references the underlying `ObjectRef`s it is based on.

Suggested structure:
- `Walkthrough { walkthrough_id, title, rev, nodes, edges, source? }`
- `WalkthroughNode { node_id, title, body_md?, refs: [ObjectRef], tags?, status? }`
- `WalkthroughEdge { from_node_id, to_node_id, kind, label? }`

Recommended behavior:
- Nodes should stay short in-diagram (title); longer context lives in the inspector (`body_md`).
- Walkthroughs can reference multiple diagrams, including cross-diagram routes (see §5).
- Walkthrough nodes may optionally store a lightweight `query_hint` describing how they were derived (so an agent can refresh them after changes without re-explaining from scratch).

---

## 3) Stable references: `ObjectRef`

### 3.1 Canonical form

All MCP tools and UI navigation should accept a canonical object reference:

```
d:<diagram_id>/<category>/<object_id>
```

Examples:
- `d:7b1d.../seq/participant/p:alice`
- `d:7b1d.../seq/message/m:0042`
- `d:91aa.../flow/node/n:authorize`
- `d:91aa.../flow/edge/e:13`

Notes:
- `diagram_id` is stable across renames.
- `object_id` is stable within a diagram; do not recycle IDs.

### 3.2 Aliases (optional convenience)

Human-friendly aliases can be supported and resolved to canonical refs:
- `name:<diagram_name>#node:<mermaid_id>`
- `name:<diagram_name>#participant:<name>`

Aliases must be **best-effort** and may become ambiguous; canonical refs must always work.

---

## 4) Cross-diagram links (XRefs)

### 4.1 Purpose

Cross-diagram links are **semantic/navigation** connections, not rendered edges:
- map a flowchart node/edge to one or more sequence messages/blocks
- record “implementation details” traces across diagrams
- enable “walk and explore” across the session

### 4.2 XRef model

```
XRef {
  xref_id,
  from: ObjectRef,
  to: ObjectRef,
  kind,        // e.g. "implements", "expands", "relates"
  label?,      // optional free text
  status       // ok | dangling_from | dangling_to | dangling_both
}
```

### 4.3 Dangling refs (TODO-style)

Dangling refs are allowed and **must be explicitly flagged**.

Requirements:
- If an xref endpoint cannot be resolved, its status reflects this.
- Dangling xrefs must be trivially retrievable for “TODO” workflows.

Examples:
- `xref.list({ status: "dangling_*" })`
- `xref.list({ dangling_only: true })`

---

## 5) Derived session meta-graph (for routes)

To support cross-diagram route finding, the session can derive a **meta-graph**:
- nodes: diagram roots + all objects addressable by `ObjectRef`
- edges:
  - flowchart edges (`from_node -> to_node`)
  - sequence adjacency (message order, block containment)
  - xrefs (bidirectional or typed direction)

This enables:
- “find a route from flow start to sequence event”
- “walk neighbors from current selection”
- “map impacted areas” for a proposed change

---

## 6) Revisions, deltas, and conflict handling

### 6.1 Revisions

Each diagram has `rev: u64` incremented on every applied mutation.

Decision:
- A single `diagram.apply_ops(...)` call is one mutation “commit”; `rev` increments by **1** if at least one op applies, otherwise it remains unchanged.

### 6.2 Base revision gating

Mutation APIs accept `base_rev`:
- if `base_rev == current_rev`: apply
- else: return a conflict payload with the current `rev` and minimal context to recover (e.g. digest + changed refs)

### 6.3 Delta API (preferred agent refresh)

Instead of re-fetching full AST, the agent uses:
- `diagram.get_delta(diagram_id, since_rev)` → structured list of changes:
  - added refs
  - removed refs
  - updated refs (field-level if feasible)

This is the default mechanism to prevent repeated re-reasoning.

---

## 7) Persistence: `.mmd`, text renders, and sidecars

### 7.1 Goals

- Load/save Mermaid `.mmd` from disk.
- Export rendered text diagrams (Unicode allowed) to a text file.
- Preserve stable IDs and xrefs even though Mermaid syntax doesn’t naturally carry IDs for all objects (especially sequence messages).

### 7.2 Recommended on-disk layout (per session)

A session can be represented as a folder containing:

- `nereid-session.meta.json` — session ID, active diagram, UI hints (optional), and shared xrefs
- `diagrams/` — one subfolder or set of files per diagram
- `walkthroughs/` — shareable walkthrough artifacts (see below)

All paths stored in session/diagram metadata must be **relative to the session folder** so the user can rename/move the folder without breaking references.

Example:

```
my-session/
  nereid-session.meta.json
  diagrams/
    auth-flow.mmd
    auth-flow.ascii.txt
    auth-flow.meta.json
    payment-seq.mmd
    payment-seq.ascii.txt
    payment-seq.meta.json
  walkthroughs/
    invite-acceptance.wt.json
    invite-acceptance.ascii.txt
```

### 7.3 Recommended storage (per diagram)

For each diagram:
- `<name>.mmd` — Mermaid interchange/export
- `<name>.ascii.txt` — rendered text output export (legacy filename; Unicode allowed)
- `<name>.meta.json` — stable IDs, xrefs, settings, and reconciliation hints

The `.meta.json` sidecar is the authoritative carrier of:
- stable internal IDs (`object_id`)
- mapping from Mermaid-visible identifiers (`mermaid_id`, participant names, edge labels) to internal IDs
- xrefs and dangling status
- last export/import digests for reconciliation

### 7.4 Import strategy

On import:
1. Parse `.mmd` → AST (semantic content).
2. If `.meta.json` exists, reconcile stable IDs by matching “structural fingerprints”:
   - flow nodes: `(mermaid_id?, label, shape)` + adjacency hints
   - flow edges: `(from, to, label, style)`
   - sequence participants: `name`
   - sequence messages: `(from, to, kind, text)` + local neighborhood + order hints
3. Any unmatched objects get new stable IDs.

### 7.5 Save/export strategy

On save/export:
- Generate canonical `.mmd` from AST (formatting may change).
- Generate `.ascii.txt` text render from renderer on every save (export-on-save).
- Update `.meta.json` with:
  - object ID mappings
  - xrefs + dangling status
  - export hashes/digests

Walkthrough save/export:
- Walkthroughs are saved to `walkthroughs/*.wt.json` inside the session folder.
- A walkthrough text export can be generated on save as `walkthroughs/*.ascii.txt` (legacy filename; shareable snapshot).

---

## 8) MCP tool surface (minimal, composable)

### 8.1 Target resolution (multi-diagram sessions)

Rules:
- If a tool call omits `diagram_id`, it targets `session.active_diagram_id`.
- If there is no active diagram, calls that require a diagram **must** include `diagram_id` (error otherwise).
- Calls that use `ObjectRef` are always unambiguous (diagram is encoded in the ref).

### 8.2 Session tools

- `diagram.list() -> { diagrams:[{diagram_id, name, kind, rev}], context }`
- `diagram.current() -> { active_diagram_id?, context }`
- `diagram.open(diagram_id) -> { active_diagram_id }`
- `route.find(from_ref, to_ref, limit, max_hops) -> { routes:[Route] }`

### 8.3 UI context tools (optional but useful)

- `attention.human.read() -> { object_ref?, diagram_id?, context }`
- `attention.agent.read() -> { object_ref?, diagram_id?, context }`
- `follow_ai.read() -> { enabled, context }`
- `selection.read() -> { object_refs:[], context }`
- `view.read_state() -> { active_diagram_id?, scroll, panes, context }`

`context` is shared read metadata:
`{ session_active_diagram_id?, human_active_diagram_id?, human_active_object_ref?, follow_ai?, ui_rev?, ui_session_rev? }`

### 8.4 Diagram read tools (small by default)

- `diagram.stat(diagram_id?) -> { rev, counts, key_names, context }`
- `diagram.diff(diagram_id, since_rev) -> { from_rev, to_rev, changes[] }`
- `diagram.get_slice(diagram_id, center_ref, radius|depth, filters) -> Subgraph`
- `diagram.get_ast(diagram_id)` (explicit; potentially large)
- `diagram.read(diagram_id?) -> { rev, kind, mermaid, context }` (explicit; potentially large)
- `diagram.render_text(diagram_id?) -> { text, context }` (explicit; potentially large)

### 8.5 Diagram mutation tools (structured ops)

Mutations are a list of typed operations applied against `base_rev`:
- `diagram.propose_ops(diagram_id, base_rev, ops[]) -> Proposal`
- `diagram.apply_ops(diagram_id, base_rev, ops[]) -> { new_rev, applied, delta }`

Ops are type-specific, examples:
- sequence:
  - `seq.add_participant { name }`
  - `seq.add_message { from, to, kind, text, after_message_id? }`
  - `seq.update_message { message_id, patch }`
  - `seq.remove_message { message_id }`
- flowchart:
  - `flow.add_node { mermaid_id?, label, shape }`
  - `flow.add_edge { from_node_id, to_node_id, label?, style? }`
  - `flow.update_node { node_id, patch }`
  - `flow.remove_node { node_id }`
- xrefs:
  - `xref.add { from_ref, to_ref, kind, label? }`
  - `xref.remove { xref_id }`

### 8.6 “Ask graph” query tools (typed primitives)

Flowchart:
- `flow.reachable(diagram_id, from_node_id, direction) -> [node_id]`
- `flow.paths(diagram_id, from_node_id, to_node_id, limit, mode) -> [Path]`
- `flow.cycles(diagram_id) -> [Cycle]`
- `flow.unreachable(diagram_id, start_node_id?) -> [node_id]`
- `flow.dead_ends(diagram_id) -> [node_id]`

Sequence:
- `seq.messages(diagram_id, filter) -> [message_id]`
- `seq.trace(diagram_id, from_message_id?, direction, limit) -> [message_id]`
- `seq.search(diagram_id, text|regex) -> [object_ref]`

Cross-diagram:
- `xref.list(filter) -> [XRef]`
- `xref.neighbors(object_ref, direction?) -> [ObjectRef]`

### 8.7 Walkthrough tools (recommended)

Walkthrough tooling is intentionally similar to diagrams: small by default (digest/delta), with structured ops for mutation.

- `walkthrough.list() -> { walkthroughs:[{walkthrough_id, title, rev}], context }`
- `walkthrough.stat(walkthrough_id) -> { digest:{ rev, counts }, context }`
- `walkthrough.get_delta(walkthrough_id, since_rev) -> { from_rev, to_rev, changes[] }`
- `walkthrough.read(walkthrough_id) -> { walkthrough:{...}, context }`
- `walkthrough.get_node(walkthrough_id, node_id) -> { node:{ title, body_md?, refs[], status? }, context }`
- `walkthrough.apply_ops(walkthrough_id, base_rev, ops[]) -> { new_rev, applied, delta }`
- `walkthrough.render_text(walkthrough_id) -> { text, context }` (explicit; potentially large)

Notes:
- Avoid a single “natural language query” tool as the primary mechanism.
- If a `diagram.query_nl(question)` exists, it should be layered on top of typed primitives.

---

## 9) Human ↔ agent collaboration loop (probe → refine → apply)

### 9.1 Default loop

1. Human focuses/selects an object in the TUI.
2. Agent fetches `selection.read()` and `diagram.stat()` (small).
3. Agent probes with a small number of typed queries/slices.
4. Agent proposes a minimal set of structured `ops[]`.
5. Human approves (optional gating), then `diagram.apply_ops`.
6. Agent refreshes via `diagram.diff(since_rev)` (not full AST).

### 9.2 Walk/explore mode

Agent performs stepwise navigation:
- start at `selection_ref`
- `xref.neighbors` / `diagram.get_slice`
- record breadcrumbs (optional notes/xrefs)
- stop when ambiguity is resolved

### 9.3 Find/map routes mode

Agent uses:
- `session.routes(from_ref, to_ref, ...)`
- then requests localized slices along the route for explanation/edit proposals

### 9.4 Walkthrough mode (shareable drill-down)

Goal: create a shareable, annotated diagram that explains "how this works" in a way that is easy to navigate.

Typical flow:
1. Human asks for an explanation ("walkthrough") anchored on a diagram/object.
2. Agent uses typed primitives (`get_digest`, `get_slice`, `paths`, `trace`, `routes`, `xref.*`) to build a high-level map.
3. Agent materializes the result into a `Walkthrough`:
   - nodes contain short titles + inspector context
   - nodes link to the underlying `ObjectRef`s as evidence
4. Human clicks nodes to drill down; agent adds more nodes/edges as needed (incremental refinement).
5. Walkthrough is saved as a shareable artifact inside the session folder.

### 9.5 Takeaways from “project walkthrough” patterns (applied to diagrams)

These patterns (popularized for agents exploring *codebases*) apply cleanly to an AST/diagram world:

- **Probe → refine**: start from `get_digest`/`get_delta`, then request localized `get_slice`/typed queries; avoid full AST dumps.
- **Breadcrumbs are artifacts**: record the exploration path as a `Walkthrough` (nodes/edges) so future turns don’t re-derive context.
- **Evidence-first checkpoints**: every step anchors to `ObjectRef`s (and optionally the typed query hints used to derive it).
- **Resumable navigation**: store last active diagram/walkthrough and selection context so agent+human can resume mid-exploration.
- **Explicit TODO surfacing**: unresolved mappings become dangling XRefs and/or walkthrough node `status`, making follow-up trivial.

---

## 10) Core question set (acceptance targets)

The system should enable the agent to answer these without re-deriving diagrams from scratch.

### 10.1 Sequence diagrams

- Who are the participants? Which never interact?
- What is the first/last message? What is the “main trace”?
- Show all messages between A and B.
- Find messages matching a text/regex query.
- What happens immediately before/after message X?
- What are the longest interaction chains (by message order)?

### 10.2 Flowcharts

- What are start nodes and terminal nodes?
- Is anything unreachable from start?
- Are there cycles/loops? Where?
- Show paths from X to Y (shortest + a few alternates).
- Which nodes have highest fan-in/fan-out?
- What are the main branches/decisions and their labels?

### 10.3 Cross-diagram

- For this flow node/edge, which sequence messages/blocks implement it?
- Along a given flow path, which sequence events are implicated (via xrefs)?
- Find a route from flow start to a specific sequence message through xrefs.
- List “TODO” mappings (dangling xrefs) for triage.

### 10.4 Walkthroughs (narrative output)

- Generate a walkthrough that explains a selected object/route at a high level, then supports drill-down into details.
- Walkthrough nodes must reference the underlying `ObjectRef`s they summarize (evidence-first).
- Walkthroughs should remain easy to refresh after edits by relying on deltas/slices rather than full re-derivation.

---

## 11) Resolved questions (decisions)

These were previously open questions; decisions are now locked (see §1.1).

1. **Mermaid round-tripping**
   - Is `.mmd` canonical-generated (formatting may change), or should we attempt format-preserving imports/edits?
   - Decision: canonical-generated; formatting changes are acceptable.

2. **Export policy**
   - Export ASCII on every save, or only on explicit export?
   - Decision: export on every save.

3. **XRefs in UI**
   - Are xrefs first-class in the UI (panel + jump list), or primarily agent-facing until queried?
   - Decision: first-class in the UI.
