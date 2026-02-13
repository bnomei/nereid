# Requirements — 24-mcp-query-extensions

This spec extends the MCP query surface for better “feature complete” workflows:

1. `xref.list` filtering (triage + navigation)
2. `session.routes` returns multiple routes (k-shortest simple paths) + output ordering control
3. `seq.search` supports regex + optional case-insensitive matching
4. Flow “fan-in/fan-out” stats (`flow.degrees`)

Protocol reference (mayor-only): `docs/protocol-01.md` §8, §10.

## Requirements (EARS)

### R1 — XRef listing filters

- THE SYSTEM SHALL support filtering `xref.list` results by:
  - `status` (`ok`, `dangling_from`, `dangling_to`, `dangling_both`, `dangling_*`)
  - `kind`
  - endpoint filters (`from_ref`, `to_ref`) and/or `involves_ref` (matches either endpoint)
  - `label_contains` (substring match)
  - `limit` (deterministic truncation)

### R2 — Multiple session routes

- THE SYSTEM SHALL return up to `limit` simple routes for `session.routes`.
- THE SYSTEM SHALL support `max_hops` as a hard cap on returned route length (hops).
- THE SYSTEM SHALL allow controlling the **output ordering** of returned routes via a parameter, defaulting to **fewest hops first**.
- THE SYSTEM SHALL keep route discovery deterministic under identical inputs.

### R3 — Sequence search regex

- THE SYSTEM SHALL support `seq.search` in `regex` mode, matching over message text.
- THE SYSTEM SHALL support optional case-insensitive matching for `seq.search` (`case_insensitive: bool`).
- WHEN a regex is invalid THE SYSTEM SHALL return `invalid_params` with an actionable error.

### R4 — Flow fan-in/fan-out stats

- THE SYSTEM SHALL provide a typed MCP tool `flow.degrees` that enables answering:
  - “Which nodes have the highest fan-in/fan-out?”
  - “What are the main branch points (high fan-out)?”
- The tool SHOULD be small-by-default (bounded output) and deterministic.
