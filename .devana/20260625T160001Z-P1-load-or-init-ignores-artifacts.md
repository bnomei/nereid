DEVANA-FINDING: v1
Priority: P1 | Confidence: high | Security-sensitive: no | Status: open
Location: src/store/session_folder.rs:704-715 | Slug: load-or-init-ignores-artifacts

# load_or_init_session replaces session when meta is missing despite existing diagram files

## Finding

If `nereid-session.meta.json` is missing but `diagrams/*.mmd` (and sidecars) already exist, `load_or_init_session` treats the folder as empty, seeds a hard-coded demo session, and saves it—dropping prior diagrams from the session index.

## Violated Invariant Or Contract

Startup must not silently reset the session index while durable diagram artifacts remain on disk; it should reconstruct from artifacts or fail loudly.

## Oracle

`load_session` loads diagrams listed in meta; `legacy_meta_without_walkthrough_ids_scans_directory` scans walkthroughs only when meta exists. `main.rs` calls `load_or_init_session()` on every start.

## Counterexample

1. Folder has `diagrams/d1.mmd` from prior work. 2. `nereid-session.meta.json` is deleted or never written. 3. `load_or_init_session()` hits `NotFound` on meta. 4. `initial_session()` creates seed "flow"/"Hello" diagram. 5. `save_session` writes new meta indexing only the seed; `d1` is orphaned on disk and absent from the session.

## Why It Might Matter

Accidental meta deletion, partial writes, or external tooling that removes meta causes silent data loss from the user's session view on next launch.

## Proof

Control-flow trace: `load_or_init_session` → `load_meta` `NotFound` → `initial_session()` → `save_session` with no scan of `diagrams/` or reconciliation of existing `.mmd` files (`704-712`).

## Counterevidence Checked

`load_or_init_session_creates_seed_diagram_when_meta_is_missing` only covers an empty folder. No code path scans `diagrams/` when meta is absent.

## Suggested Next Step

On meta `NotFound`, scan `diagrams/` (and walkthroughs) and rebuild meta, or return a distinct error requiring repair instead of seeding.

DEVANA-KEY: src/store/session_folder.rs:704-715 | P1 | load-or-init-ignores-artifacts
DEVANA-SUMMARY: P1 high src/store/session_folder.rs:704-715 - Missing session meta triggers seed session creation, ignoring existing diagram files on disk.