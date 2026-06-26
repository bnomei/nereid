DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: open
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

DEVANA-KEY: src/store/session_folder.rs:771-835,893 | P1 | save-session-diagram-non-atomic
DEVANA-SUMMARY: P1 high src/store/session_folder.rs:771-835,893 - Partial save_session failure can leave new .mmd on disk while meta and sidecar still describe the old revision.