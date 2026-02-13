# Design — 16-audit-remediation

Source of truth for this spec’s backlog is `docs/audit.md` (Implementation Audit dated 2026-02-07).

Many of the highest-severity findings in that document are already resolved in the current codebase (store symlink defenses + atomic writes, xref persistence, MCP delta history + snapshot recovery, seq endpoint validation, routing no-panics, etc.). This spec focuses on the **remaining** correctness/robustness gaps and on eliminating current Clippy warnings.

## Audit finding status map (2026-02-07 audit)

Already resolved (keep regression tests; no new work unless regressions are found):
- Store symlink defense + atomic writes.
- Persist/load `Session.xrefs`, `Session.active_walkthrough_id`, `Diagram.rev`.
- MCP delta sync recovery (bounded history) + snapshot recovery tool.
- Seq op validation for participant endpoints.
- Routing hard-crash/panic paths removed (fallback errors instead).
- Walkthrough ASCII export wired to a real renderer.
- Sequence export rejects invalid/newline message text.
- `SeqOp::UpdateParticipant` preserves unrelated metadata.
- DeltaBuilder add→remove spurious “removed” cancelled in-batch.

Still open (work items in `tasks.md`):
- Walkthrough deletion is not representable (stale `walkthroughs/*.wt.json` can resurrect).
- Walkthrough `rev` restore is O(rev) and should be O(1) with a safety cap.
- File-name portability for persisted artifacts (encode ids into safe file names without changing model/protocol ids).
- Performance follow-ups called out in the audit (routing scaling, TUI allocations, session route derivation reuse).
- Clippy warnings (currently 40 warnings across store/mcp/tui/render/query/ops/format).

## Walkthrough deletion is representable

Problem: `save_session()` only writes walkthroughs that exist in memory, but it doesn’t remove stale files. `load_session()` scans `walkthroughs/*.wt.json` and loads everything it finds.

Design:
- Extend `nereid-session.meta.json` to include a `walkthrough_ids: [String]` list.
- On save: write `walkthrough_ids` from the in-memory `Session.walkthroughs()` keys, and optionally garbage-collect stale `walkthroughs/*.wt.json` + `*.ascii.txt` that are not referenced.
- On load:
  - If `walkthrough_ids` is present: load exactly those walkthroughs.
  - If absent (backwards compat): fall back to directory scan as today.

## Walkthrough `rev` restore (O(1) + safety cap)

Problem: `walkthrough_from_json()` restores `rev` by looping `bump_rev()` `rev` times.

Design:
- Add a model-level setter (e.g. `Walkthrough::set_rev(u64)`) mirroring `Diagram::set_rev`.
- In store loading:
  - Validate/cap `rev` to a reasonable max (defense-in-depth for hostile/corrupt files).
  - Set `rev` directly instead of looping.

## Portable filenames for persisted artifacts

Problem: model ids are permissive and some are used as file name segments (not portable on Windows).

Design:
- Keep IDs unchanged in the model/protocol.
- Introduce a store-local encoding function to map ids → safe filename segments (e.g., percent-encoding for Windows-disallowed characters).
- Keep on-disk paths source-of-truth in `nereid-session.meta.json` (so existing sessions remain loadable).
- For APIs that derive paths from ids (e.g. `load_walkthrough(walkthrough_id)`), support both encoded and legacy filenames for backwards compatibility.

## Clippy cleanup strategy

Approach:
- Fix warnings by module/file (prefer mechanical refactors with no behavior changes).
- For “design-ish” warnings (e.g. `clippy::result_large_err`), prefer structural fixes (boxing/wrappers) over blanket `#[allow]`, unless the refactor would be excessively invasive.

Validation gates:
- `cargo test --offline`
- `cargo clippy --offline`

## Performance follow-ups

Keep changes deterministic and test-backed:
- Flowchart routing: prefer local optimizations (allocation reuse, cheaper maps/sets with deterministic sorting at boundaries) before algorithmic rewrites.
- TUI: remove per-frame allocations where possible; borrow `&str` and cache filtered indices/labels where safe.
- Session route derivation: provide an optional pre-derived adjacency representation for callers that need repeated queries.
