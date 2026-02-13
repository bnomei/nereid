# Specs index

This repo tracks the plan and execution for a Rust TUI app that supports collaborative creation/reasoning over Mermaid-like **sequence diagrams** and **flowcharts**, rendered as **ASCII/Unicode** from a stable **AST**.

Protocol/design reference: `docs/protocol-01.md` (mayor-only; worker tasks must include extracted `Context:` excerpts instead of pointing workers at this doc)

## Specs

- `01-diagram-tui-rust` — Bootstrap + initial implementation (crate skeleton + stable IDs/ObjectRef)
- `02-dependencies` — Cargo dependencies + feature flags (owns `Cargo.toml`)
- `03-model-core` — Core data model (owns `src/model/`)
- `04-ops-delta` — Structured ops + revision/delta plumbing (owns `src/ops/`)
- `05-mermaid-format` — Mermaid subset parse/export (owns `src/format/`)
- `06-render-canvas` — Canvas + Unicode drawing primitives (owns `src/render/`)
- `07-layout-engine` — Layout + routing (owns `src/layout/`)
- `08-query-engine` — Typed “ask graph” primitives + routing (owns `src/query/`)
- `09-session-store` — Session folder persistence (owns `src/store/`)
- `10-walkthroughs` — Walkthrough artifacts + persistence (cross-cutting; later)
- `11-tui` — `ratatui` UI (owns `src/tui/`)
- `12-mcp` — MCP server + tool surface (owns `src/mcp/`)
- `13-diagram-renderers` — Diagram renderers (sequence/flow to ASCII/Unicode) (owns `src/render/`)
- `14-cli-entrypoint` — CLI entrypoint + argument parsing (owns `src/main.rs`)
- `15-audit` — Implementation audit report (owns `docs/audit.md`)
- `16-audit-remediation` — Close remaining audit issues + clippy warnings (cross-cutting)
- `17-tui-perf` — Reduce per-frame allocations (cross-cutting)
- `18-mcp-writeback` — Persist MCP mutations to session folder (CLI `--mcp --session <dir>`)
- `19-mcp-protocol-surface` — Complete remaining MCP protocol tools (`docs/protocol-01.md` §8)
- `20-walkthrough-mcp-mutation` — Walkthrough MCP delta + mutation tools (`docs/protocol-01.md` §8.7)
- `21-mcp-persist-session-active` — Persist active diagram/walkthrough setters in MCP persistent mode
- `22-mcp-ui-context-and-session-routes` — Add missing protocol tools (`session.routes`, `ui.get_selection`, `ui.get_view_state`)
- `23-diagram-meta-roundtrip` — Diagram `.meta.json` sidecar round-trip (stable ids + non-Mermaid fields)
- `24-mcp-query-extensions` — MCP query upgrades (xref filters, multi-routes, regex search, flow stats)
- `25-render-helper-dedup` — Deduplicate render helper functions (truncate/len/canvas trim) (owns `src/render/`)
- `26-flow-reachability-unification` — Unify flow reachability logic (query engine as source of truth) (owns `src/query/flow.rs`, `src/mcp/server.rs`)
- `27-text-render-naming` — Fix “ascii” naming mismatch for Unicode text renders (owns `src/mcp/`, `src/store/`)
- `28-session-routes-adjacency` — Expand session route adjacency to include `seq/participant` + `flow/edge` (owns `src/query/session_routes.rs`)
- `29-ui-highlights` — Transient object highlighting (TUI + MCP) (owns `src/render/`, `src/tui/`, `src/mcp/`, `src/main.rs`)
- `30-perf-persistence-incremental-save` — Speed up session persistence (save/writeback) with benchmark gating (owns `src/store/`, `src/mcp/`, `src/main.rs`)
- `31-perf-flow-routing` — Speed up orthogonal flow edge routing with benchmark gating (owns `src/layout/flowchart.rs`)
- `32-perf-flow-render-large-labels` — Speed up flow rendering for long labels with benchmark gating (owns `src/render/flowchart.rs`)
- `33-perf-save-session-compute` — Speed up compute-only portion of `save_session` with benchmark gating (owns `src/store/`, `src/format/`, `src/layout/`, `src/render/`)
- `34-perf-flow-layout` — Speed up flow layout with benchmark gating (owns `src/layout/flowchart.rs`)
- `35-mermaid-ascii-parity-rendering` — Mermaid-ascii parity subset (flow decorations + sequence aliases/returns) (owns `src/render/`, `src/format/mermaid/`, `src/model/`)

## High-level dependencies

- `03-model-core` depends on `spec:01-diagram-tui-rust/T002` (IDs + `ObjectRef`)
- `04-ops-delta` depends on `spec:03-model-core/T001`
- `05-mermaid-format` depends on `spec:03-model-core/T001`
- `07-layout-engine` depends on `spec:03-model-core/T001`, `spec:05-mermaid-format/T001`, `spec:06-render-canvas/T001`
- `08-query-engine` depends on `spec:03-model-core/T001`
- `09-session-store` depends on `spec:02-dependencies/T001`, `spec:03-model-core/T001`, `spec:05-mermaid-format/T001`, `spec:06-render-canvas/T001`
- `11-tui` depends on `spec:02-dependencies/T002`, `spec:03-model-core/T001`, `spec:06-render-canvas/T001`, `spec:07-layout-engine/T001`
- `12-mcp` depends on `spec:02-dependencies/T005`, `spec:03-model-core/T001`, `spec:04-ops-delta/T001`, `spec:08-query-engine/T001`
- `13-diagram-renderers` depends on `spec:03-model-core/T001`, `spec:06-render-canvas/T001`, `spec:07-layout-engine/T001`
- `14-cli-entrypoint` depends on `spec:11-tui/T001`
- `15-audit` depends on: (none; review-only)
- `16-audit-remediation` depends on: `spec:15-audit/T005`
- `17-tui-perf` depends on: `spec:11-tui/T001`
- `18-mcp-writeback` depends on: `spec:09-session-store/T006`, `spec:12-mcp/T023`, `spec:14-cli-entrypoint/T003`
- `19-mcp-protocol-surface` depends on: `spec:12-mcp/T023`
- `20-walkthrough-mcp-mutation` depends on: `spec:19-mcp-protocol-surface/T003`, `spec:10-walkthroughs/T003`, `spec:18-mcp-writeback/T001`
- `21-mcp-persist-session-active` depends on: `spec:18-mcp-writeback/T001`, `spec:20-walkthrough-mcp-mutation/T001`
- `22-mcp-ui-context-and-session-routes` depends on: `spec:12-mcp/T023`
- `23-diagram-meta-roundtrip` depends on: `spec:09-session-store/T006`
- `24-mcp-query-extensions` depends on: `spec:12-mcp/T023`, `spec:08-query-engine/T001`
- `25-render-helper-dedup` depends on: `spec:06-render-canvas/T001`, `spec:13-diagram-renderers/T001`
- `26-flow-reachability-unification` depends on: `spec:08-query-engine/T002`, `spec:12-mcp/T020`
- `27-text-render-naming` depends on: `spec:12-mcp/T015`, `spec:12-mcp/T012`, `spec:09-session-store/T004`, `spec:10-walkthroughs/T003`
- `28-session-routes-adjacency` depends on: `spec:08-query-engine/T003` (related: `spec:24-mcp-query-extensions/T002`)
- `29-ui-highlights` depends on: `spec:11-tui/T001`, `spec:12-mcp/T023`, `spec:13-diagram-renderers/T001`
