DEVANA-FINDING: v1
DEVANA-STATE: fixed
Priority: P1 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/store/session_folder.rs:771-835,893 | Slug: save-session-diagram-non-atomic

# save_session can leave diagram .mmd and sidecar/meta out of sync on partial failure

## Finding

`save_session` writes each diagram's `.mmd`, sidecar `.meta.json`, and session meta in sequence without rollback. If an intermediate step fails, disk can hold a new `.mmd` while session meta and sidecar still describe the previous revision.

## Violated Invariant Or Contract

A failed `save_session` must not leave diagram artifacts in a mixed state that `load_session` reconciles differently from the session that initiated the save.

## Oracle

Per-diagram writes are ordered: `export_diagram_mmd` then `save_diagram_meta` then later `save_meta` (`771-835`, `893`). Each step returns independently; no transaction spans the multi-file commit.

## Counterexample

1. Sequence diagram at rev 5 with populated sidecar (stable message/participant IDs). 2. In-memory mutation bumps rev to 6. 3. `save_session` succeeds at `export_diagram_mmd` but `save_diagram_meta` fails (I/O). 4. `save_meta` never runs; session meta still says rev 5. 5. Restart: `load_session` parses new `.mmd` but reconciles against old sidecar → stable IDs and notes can diverge from pre-crash session and from xrefs/selection keyed on those IDs.

## Why It Might Matter

Crash or I/O failure mid-save corrupts cross-file consistency; reload produces a session that neither matches pre-save disk nor the in-memory state that triggered the save.

## Proof

Ordered side effects with independent error returns: `export_diagram_mmd` (`771-772`) → `save_diagram_meta` (`826-835`) → `save_meta` (`893`); failure after `.mmd` write yields cross-file skew without rollback.

## Counterevidence Checked

Distinct from `save-session-gc-before-meta` (walkthrough GC ordering). Retrying `save_session` with the same in-memory session can heal if the process stays up; does not help after restart with skewed disk only.

## Suggested Next Step

Write to temp files and rename atomically per diagram, or defer `.mmd` export until sidecar and meta can commit together; add rollback on failure.

## Status Notes

2026-06-26: Still open after static validation. The sidecar-first ordering blocks the original "new `.mmd` plus stale sidecar" direction, but the multi-file save remains non-atomic in the reverse direction: if the sidecar write succeeds and `export_diagram_mmd` fails, disk is left with an old `.mmd` and a new sidecar while session meta has not committed. `load_session` can still reconcile old Mermaid content through a sidecar from a different revision.

2026-06-26: Marked fixed after adding diagram artifact snapshots and rollback around changed `.mmd`/sidecar writes before the session meta commit. A failed `.mmd` write now restores the just-written sidecar, later pre-meta failures roll back all changed diagram artifacts, and diagram ASCII export scheduling is deferred until after `save_meta` succeeds. Regressions cover sidecar write failure, `.mmd` write failure after sidecar success, and session meta write failure after diagram artifacts were written.

DEVANA-KEY: src/store/session_folder.rs:771-835,893 | P1 | save-session-diagram-non-atomic
DEVANA-SUMMARY: fixed P1 high src/store/session_folder.rs:771-835,893 - Partial save_session failure can leave new .mmd on disk while meta and sidecar still describe the old revision.
