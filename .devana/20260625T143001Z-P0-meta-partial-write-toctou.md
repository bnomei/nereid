DEVANA-FINDING: v1
Priority: P0 | Confidence: high | Security-sensitive: no | Status: open
Location: src/store/session_folder.rs:1155-1181 | Slug: meta-partial-write-toctou

# Partial meta updates use load-modify-save without concurrency guard

## Finding

`save_selected_object_refs` and `save_active_diagram_id` read the full session meta, patch one field, and write the full meta back. Concurrent `save_session` calls that add diagrams or walkthroughs between the read and write are silently dropped from the meta index.

## Violated Invariant Or Contract

Partial meta field updates must not erase fields written by a concurrent full session save. Meta is the session index of record.

## Oracle

`save_session` (`src/store/session_folder.rs:759-894`) rebuilds and writes complete meta including `diagrams`, `walkthrough_ids`, and `xrefs`. Partial helpers must be atomic relative to that.

## Counterexample

1. Process A: `load_meta()` → meta v1 lists diagram `d1` only.
2. Process B: `save_session()` → meta v2 lists `d1` and `d2`; `diagrams/d2.mmd` written.
3. Process A: `save_active_diagram_id()` writes meta v1 plus new `active_diagram_id`, dropping `d2` from index.
4. `load_session()` returns session without `d2` while `diagrams/d2.mmd` remains on disk.

## Why It Might Matter

TUI and MCP both call these helpers (`src/tui/mod.rs`, `src/mcp/server/collaboration.rs`) while the other side may be adding diagrams. Orphan diagram files and invisible diagrams in the UI/MCP tools.

## Proof

**State transition mismatch:** `load_meta` → mutate one field → `save_meta` with no version check or merge.

**Dataflow trace:** `save_active_diagram_id` at 1170-1175 replaces entire `SessionMeta` JSON; concurrent `save_session` at 893 also writes full meta — last writer wins on the whole document.

## Counterevidence Checked

Per-file diagram writes are atomic (`write_atomic_in_session_inner`). `src/mcp/server/tests.rs:4098-4133` tests sequential external selection then MCP add, not concurrent interleaving. No optimistic-locking field on meta.

## Suggested Next Step

Add meta generation/version field and retry on conflict, or route all meta mutations through a single serialized writer that merges fields.

DEVANA-KEY: src/store/session_folder.rs:1155-1181 | P0 | meta-partial-write-toctou
DEVANA-SUMMARY: P0 high src/store/session_folder.rs:1155-1181 - save_active_diagram_id and save_selected_object_refs can erase concurrent save_session additions via unguarded load-modify-save on session meta.