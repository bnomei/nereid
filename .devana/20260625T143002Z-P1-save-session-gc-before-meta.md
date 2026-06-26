DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: open
Location: src/store/session_folder.rs:887-893 | Slug: save-session-gc-before-meta

# save_session deletes walkthrough files before meta commit

## Finding

`save_session` garbage-collects removed walkthrough files before calling `save_meta`. If `save_meta` fails after GC, the meta index still references deleted walkthrough files and `load_walkthrough` returns `NotFound`.

## Violated Invariant Or Contract

Session meta is the index of record. If a save fails, on-disk state must remain loadable and consistent with meta.

## Oracle

`load_session` / `load_walkthrough` expect every `walkthrough_id` in meta to have a corresponding JSON file. `save_session` should commit as a unit or roll back side effects.

## Counterexample

1. Session removes walkthrough `w2` from in-memory state.
2. `save_session` runs `garbage_collect_walkthrough_files` → deletes `walkthroughs/w2.wt.json`.
3. `save_meta` fails (disk full, permission denied, I/O error).
4. Meta still lists `w2`; `load_walkthrough("w2")` fails with not-found while meta claims it exists.

## Why It Might Matter

Partial persistence leaves the session folder in an unloadable or inconsistent state. Retry may not recover without manual repair.

## Proof

**Control-flow trace:** `save_session` lines 887-890 call `garbage_collect_walkthrough_files` unconditionally when walkthrough set changed; line 893 `save_meta` is separate and can fail independently.

**Dataflow trace:** GC deletes files → meta write fails → meta still references deleted paths.

## Counterevidence Checked

Per-file writes use atomic rename (`helpers.rs`). `src/store/session_folder/tests.rs` covers happy-path remove+reload only, not injected `save_meta` failure after GC.

## Suggested Next Step

Write new meta to temp file and rename after all side effects succeed, or defer GC until after successful meta commit.

DEVANA-KEY: src/store/session_folder.rs:887-893 | P1 | save-session-gc-before-meta
DEVANA-SUMMARY: P1 high src/store/session_folder.rs:887-893 - Walkthrough GC runs before save_meta, so a failed meta write leaves meta pointing at deleted walkthrough files.