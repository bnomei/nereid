# Implementation Audit (2026-02-07)

This audit covers logical bugs, missing wiring, spec/protocol misalignment, performance risks, and security/robustness concerns in the current Nereid Rust implementation.

Note: `docs/` and `specs/` are currently ignored by git via `.gitignore`.

## Status

- `cargo test --offline`: **ok** (189 tests passed)
- `cargo clippy --offline`: **ok** (0 warnings)

## Severity scale

- **P0**: crash/data loss/security issue likely in normal use
- **P1**: high-priority correctness/security gap; likely to bite soon
- **P2**: important gap or UX limitation; fix when scheduling next wave
- **P3**: cleanup/maintainability/perf polish; backlog

---

## Key findings (highest priority)

### [P1] Session store can write outside session root via symlinks

- Status: Closed (2026-02-07) — store now uses atomic writes + best-effort symlink refusal; see `write_atomic_in_session` + `src/store/session_folder.rs` tests.
- Affected: `src/store/session_folder.rs`
- Problem: The JSON fields are validated against `..`/absolute paths, but filesystem symlinks under the session folder (e.g. `diagrams/`, `walkthroughs/`, or a target file) can redirect `fs::write` outside the session root.
- Impact: Potential overwrite of arbitrary files with the user’s permissions (robustness/security).
- Suggested fix:
  - Refuse symlinked directories/files under the session root (best-effort, OS-dependent).
  - Use atomic temp-write + rename to reduce partial writes and mitigate some destination-file symlink issues (still not a full defense without OS-level “no-follow”).

### [P1] `Session::xrefs` are not persisted/loaded

- Status: Closed (2026-02-07) — `nereid-session.meta.json` now round-trips xrefs and refreshes dangling status on load.
- Affected: `src/model/session.rs`, `src/store/session_folder.rs`, `src/tui/mod.rs`
- Problem: `SessionFolder::{save_session, load_session}` never round-trips the `Session.xrefs` map. In-memory xrefs (and any “ask graph” routing that depends on them) disappear on restart.
- Impact: Misalignment with the protocol’s “xrefs are first-class” decision; UI XRef panel will be empty after loading a session even if it previously had xrefs.
- Suggested fix:
  - Persist xrefs either inside `nereid-session.meta.json` or as a dedicated file (e.g. `session.xrefs.json`) and load them on startup.
  - On load, recompute/refresh dangling status by validating endpoints against the loaded diagrams.

### [P1] MCP delta sync is non-recoverable when a client misses revisions

- Status: Closed (2026-02-07) — MCP server now maintains a bounded per-diagram delta history for recovery within a revision window.
- Affected: `src/mcp/server.rs`, `src/mcp/types.rs`
- Problem: `diagram.get_delta` is “last-delta cache only”. If `since_rev != last.from_rev` or `current_rev != last.to_rev`, the client gets `delta_unavailable` and has no way to recover via MCP.
- Impact: Any missed poll / multi-client update scenario can strand clients.
- Suggested fix:
  - Add a snapshot tool (`diagram.get_ast` / `diagram.get_snapshot`) and/or keep a small ring-buffer of deltas per diagram for a bounded revision window.

### [P1] Sequence ops can create invalid diagrams (orphan participant references)

- Status: Closed (2026-02-07) — ops validate participant ids for message add/update and return typed errors.
- Affected: `src/ops/mod.rs`
- Problem: Sequence message ops don’t validate that `from_participant_id`/`to_participant_id` exist in the sequence AST’s participant map.
- Impact: Creates invalid sequence diagrams that can break renderers/exporters and complicate client logic.
- Suggested fix:
  - Validate participant existence in `SeqOp::AddMessage` and `SeqOp::UpdateMessage` and return a typed error (mapped through MCP).

### [P1] Flowchart routing uses panic paths (hard crash)

- Status: Closed (2026-02-07) — routing now validates endpoints and returns structured errors; regression tests cover missing placements.
- Affected: `src/layout/flowchart.rs`
- Problem: Routing contains `expect`/panic-based invariants; unexpected conditions can crash the process rather than returning a recoverable error.
- Impact: A routing bug or extreme input becomes a full app/server crash.
- Suggested fix:
  - Convert routing to return `Result`/`Option` and plumb failures into render errors; fall back to baseline connectors when routing fails.

---

## Other correctness & wiring gaps

### [P2] Diagram revision (`Diagram.rev`) is not persisted (resets on load)

- Status: Closed (2026-02-07) — diagram rev is persisted in diagram meta and restored on load.
- Affected: `src/model/diagram.rs`, `src/store/session_folder.rs`
- Impact: Revision tracking is lost across restarts; any “resume deltas from disk” plan will be incorrect.
- Suggested fix: Persist diagram rev and add a non-O(rev) restore API (setter/constructor) in the model.

### [P2] `Session.active_walkthrough_id` is not persisted/loaded

- Status: Closed (2026-02-07) — active walkthrough id is persisted in session meta and restored on load.
- Affected: `src/model/session.rs`, `src/store/session_folder.rs`
- Impact: UI continuity breaks across restarts.
- Suggested fix: Add `active_walkthrough_id` to `nereid-session.meta.json` with `#[serde(default)]` for backwards compatibility.

### [P2] Walkthrough deletion is not representable (stale files can resurrect)

- Status: Closed (2026-02-07) — store now persists `walkthrough_ids` in `nereid-session.meta.json` and loads only referenced walkthroughs, preventing resurrection.
- Affected: `src/store/session_folder.rs`
- Problem: `save_session` never deletes old `walkthroughs/*.wt.json`, while `load_session` loads **all** `*.wt.json` it finds.
- Impact: Deleted walkthroughs can reappear after save/load.
- Suggested fix: Track walkthrough ids in session meta (load only known ones), and/or implement a careful GC strategy on save.

### [P3] Walkthrough `rev` restore is O(rev) and can be abused

- Status: Closed (2026-02-07) — walkthrough `rev` restore now uses an O(1) setter with a deterministic safety cap.
- Affected: `src/store/session_folder.rs`
- Problem: `walkthrough_from_json` restores rev by calling `bump_rev()` in a loop.
- Impact: Corrupted/hostile `rev` values can cause very slow loads.
- Suggested fix: Add a direct rev setter or cap `rev` on load with an error.

### [P2] Walkthrough ASCII export is a placeholder

- Status: Closed (2026-02-07) — walkthrough Unicode/ASCII rendering is implemented and exported on save.
- Affected: `src/store/session_folder.rs`
- Impact: Walkthrough export-on-save exists, but it’s not a real renderer yet.
- Suggested fix: Implement a minimal walkthrough renderer (boxes + arrows) using the existing `Canvas` primitives.

### [P2] Sequence export can emit invalid Mermaid for mutated ASTs

- Status: Closed (2026-02-07) — sequence export validates message text and fails on newlines/control chars.
- Affected: `src/format/mermaid/sequence.rs`, `src/ops/mod.rs`
- Problem: `export_sequence_diagram` does not validate/escape message text (or participant names if mutated), so ops could introduce newlines/control chars and produce invalid `.mmd`.
- Suggested fix: Validate/escape message text on export (similar to flowchart label validation) and/or validate edits at op-apply time.

### [P2] `SeqOp::UpdateParticipant` drops unrelated participant metadata

- Status: Closed (2026-02-07) — UpdateParticipant preserves role metadata (regression tested).
- Affected: `src/ops/mod.rs`, `src/model/seq_ast.rs`
- Problem: Update reconstructs `SequenceParticipant` and clears `role`.
- Suggested fix: Mutate in place or preserve `role` when rebuilding.

### [P2] DeltaBuilder can report spurious “removed” for add→remove in same batch

- Status: Closed (2026-02-07) — DeltaBuilder cancels an added entry on removal in the same batch.
- Affected: `src/ops/mod.rs`
- Suggested fix: When recording removal, cancel an outstanding “added” entry instead of adding to “removed”.

---

## Performance risks / scaling notes

### [P2] Flowchart routing cost can grow quickly

- Status: Closed (2026-02-08) — routing now reuses scratch buffers and avoids per-route/per-step allocations; deterministic stress test added.
- Affected: `src/layout/flowchart.rs`
- Problem: BFS per edge with `BTreeMap/BTreeSet` + expanding search bounds.
- Suggested fix: Consider a cheaper deterministic router, reuse allocations across routes, and/or use faster deterministic hash-based sets/maps (keeping determinism by sorting where needed).

### [P2] TUI per-frame allocation opportunities

- Status: Closed (2026-02-08) — cached visible-xref indices and borrowed labels for list items (no per-frame String clones).

- Affected: `src/tui/mod.rs`
- Examples: repeated allocation of visible xref indices, cloning labels into list items.
- Suggested fix: Cache filter results, borrow `&str` where possible, precompute lookup sets.

### [P3] Session route derivation recomputes adjacency for each query

- Status: Closed (2026-02-08) — added `SessionRouteAdjacency` + `find_route_with_adjacency(...)` to enable adjacency reuse across route queries (no change to existing `find_route`).
- Affected: `src/query/session_routes.rs`
- Suggested fix: Cache/derive once per session rev (if/when session-level rev exists).

---

## Security & robustness

### [P3] Non-atomic writes (crash can leave partial files)

- Status: Closed (2026-02-07) — store uses temp-write + rename for JSON and exported files.
- Affected: `src/store/session_folder.rs`
- Suggested fix: temp-write + rename for JSON and exported files.

### [P2] IDs are permissive; file-name portability risk

- Status: Closed (2026-02-08) — store now encodes persisted IDs into filename-safe segments; walkthrough load supports both encoded + legacy filenames (tests cover ':' case).
- Affected: `src/model/ids.rs`, `src/store/session_folder.rs`
- Problem: IDs only forbid `/`. When used as file names (e.g. `<diagram_id>.mmd`), unusual characters may cause portability issues (especially on Windows) or surprising behavior.
- Suggested fix: Either constrain ids used for persistence or encode them when constructing file names.

---

## Clippy notes (selected)

- Status: Closed (2026-02-07) — `cargo clippy --offline` is now clean (0 warnings).

---

## Recommended next wave (candidate tasks)

- (none) — all audit findings in this report are closed as of 2026-02-08.
