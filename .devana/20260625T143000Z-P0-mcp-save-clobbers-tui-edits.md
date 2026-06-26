DEVANA-FINDING: v1
Priority: P0 | Confidence: high | Security-sensitive: no | Status: open
Location: src/mcp/server/diagram.rs:974-1021 | Slug: mcp-save-clobbers-tui-edits

# MCP full-session save clobbers concurrent TUI diagram edits

## Finding

When TUI and MCP HTTP share a `SessionFolder`, MCP tool handlers sync disk once at handler entry, mutate one diagram or walkthrough, then call `save_session` with the entire in-memory session snapshot. Diagrams the TUI saved concurrently on disk are overwritten with stale rev/AST from MCP's pre-sync snapshot.

## Violated Invariant Or Contract

Concurrent writers must not regress another diagram's persisted rev or content. TUI `persist_pending_diagram_sync` rev-guards only the diagram being edited; MCP must not write stale copies of other diagrams.

## Oracle

TUI `persist_pending_diagram_sync` (`src/tui/mod.rs:1503-1515`) checks `expected_disk_rev` for the edited diagram only. MCP `diagram.apply_ops` persists via `save_session(&candidate_session)` built from `state.session.clone()` after a single `lock_state_synced` reload.

## Counterexample

1. Disk: diagrams `d-a` rev=1, `d-b` rev=1.
2. MCP calls `lock_state_synced` → in-memory session has both at rev=1.
3. TUI edits `d-a` to rev=2 and `save_session` succeeds (disk `d-a` rev=2).
4. MCP applies ops to `d-b`, then `save_session(&candidate_session)` where `candidate_session` still has `d-a` rev=1.
5. Reload from disk: `d-a` reverts to rev=1 with stale AST; TUI edit is lost.

## Why It Might Matter

Data loss on the default TUI+MCP HTTP path (`main.rs` shares one `SessionFolder`). Agents editing one diagram while a human edits another silently discard human work.

## Proof

**Cross-entry mismatch:** TUI reload path (`sync_session_from_disk`) replaces full session from disk; MCP save path writes full session without per-diagram rev reconciliation at commit time.

**Dataflow trace:** `lock_state_synced` → `sync_state_with_session_folder` (load once) → mutate target diagram → `candidate_session = state.session.clone()` → `save_session(&candidate_session)` overwrites all diagram entries in meta and mmd files.

Same pattern in `src/mcp/server/walkthrough.rs:335-405` and `src/mcp/server/xref.rs`.

## Counterevidence Checked

`sync_state_with_session_folder` reloads disk only at mutex acquisition, not immediately before `save_session`. `src/mcp/server/tests.rs` covers sequential external selection updates, not interleaved diagram-body writes. No session-level file lock in `src/store/`.

## Suggested Next Step

Re-read disk session immediately before save and merge only the mutated diagram/walkthrough, or reject save when any other diagram's disk rev differs from the in-memory copy.

DEVANA-KEY: src/mcp/server/diagram.rs:974-1021 | P0 | mcp-save-clobbers-tui-edits
DEVANA-SUMMARY: P0 high src/mcp/server/diagram.rs:974-1021 - MCP save_session writes a stale full-session snapshot that can overwrite concurrent TUI diagram saves on the shared SessionFolder.