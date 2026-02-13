# Design — 15-audit

## Approach

Split the audit into parallel passes by subsystem, then compile into a single report:

- `src/store/` persistence + path safety
- `src/mcp/` tool semantics + delta/conflict behavior
- `src/ops/` op validation + rev/delta behavior
- `src/format/` parsing/export round-tripping + input safety
- `src/layout/` + `src/render/` perf invariants + determinism
- `src/tui/` wiring + state correctness

## Output format (`docs/audit.md`)

- Short “Status” section (tests/clippy, current date).
- Findings grouped by:
  - Correctness & missing wiring
  - Protocol/spec alignment gaps
  - Performance
  - Security & robustness
  - DX/maintainability
- Each finding includes:
  - Severity: P0–P3
  - Affected files
  - Why it matters
  - Suggested fix (or a proposed task to add to a future wave)

