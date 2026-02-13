# Tasks — DRAFT-29-ui-highlights

Meta:
- Spec: DRAFT-29-ui-highlights — Transient object highlighting (TUI + MCP)
- Depends on: spec:11-tui, spec:12-mcp, spec:13-diagram-renderers
- Global scope:
  - src/render/
  - src/tui/
  - src/mcp/
  - src/main.rs

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T004: Run TUI + MCP together over streamable HTTP at `/mcp` (owner: mayor) (scope: src/main.rs, src/mcp/, src/tui/) (depends: T002,T003)
  - Context: MCP-over-stdio is incompatible with an interactive TUI. Serve RMCP streamable HTTP on `http://127.0.0.1:<port>/mcp` while the TUI is running.
  - DoD:
    - Add a CLI option (or default behavior) to start the TUI while also serving MCP streamable HTTP on `127.0.0.1`.
    - Display the bound MCP URL in the TUI status bar.
    - Ensure TUI and MCP share the same in-memory highlight state so agent-driven highlights show up in the TUI.
    - Document how to connect (short README snippet or `--help` output).
  - Validation: `cargo test` (covers streamable HTTP `tools/call` + TUI agent highlight rendering); optional manual smoke: run combined mode; call MCP highlight tool over `http://127.0.0.1:<port>/mcp`; observe highlight update in TUI.
  - Escalate if: RMCP streamable HTTP server integration requires extra infra (axum/tower, tokio `net`); document exact dependencies + rmcp feature flags before implementing.
  - Started_at: 2026-02-09T12:44:21+00:00
  - Completed_at: 2026-02-09T13:13:27+00:00
  - Completion note: TUI mode now always serves MCP over RMCP streamable HTTP at `/mcp` (on `127.0.0.1`), shows the bound URL in the status bar, and shares a runtime-only `agent_highlights` set with MCP so agent-driven highlights render live; added `--mcp-http-port` for port selection.
  - Validation result: `cargo test` (ok)

- [x] T002: TUI selection highlight (owner: mayor) (scope: src/tui/) (depends: T001)
  - Context: Selecting an item in the Objects list already yields an `ObjectRef`; use that to highlight the matching diagram region.
  - DoD:
    - Diagram pane renders with selection-highlight applied when an object is selected (even when focus is not Diagram).
    - Highlight updates as selection changes.
    - User selection highlight uses a distinct style from agent-driven highlights.
    - No crashes when selection is `None` or object has no spans in the index (e.g. unknown category).
  - Validation: `cargo test` + manual smoke: run TUI, select different objects, observe highlight.
  - Escalate if: ratatui `Paragraph` + `Text` performance is problematic; add minimal caching (per active diagram + rev).
  - Started_at: 2026-02-09T12:36:20+00:00
  - Completed_at: 2026-02-09T12:43:36+00:00
  - Completion note: Switched the diagram pane to render from the annotated `HighlightIndex` and apply selection highlights using ratatui `Text` spans (with dedicated user/agent/both styles); added unit tests to ensure highlighting is applied and safely disabled when selection is none.
  - Validation result: `cargo test` (ok)

- [x] T001: Annotated diagram render index (owner: worker:019c4239-aba1-7681-aac0-73c0f67bae56) (scope: src/render/) (depends: -)
  - Context: Highlights must be stable and cell-accurate; avoid substring matching. Output a `HighlightIndex` mapping `ObjectRef -> [LineSpan]`.
  - DoD:
    - Add new render entrypoint(s) that return `{ text, highlight_index }` for both sequence + flowchart diagrams.
    - Include spans for seq participants/messages and flow nodes/edges.
    - Add unit tests covering at least one seq + one flow fixture where spans include the expected label/connector region.
  - Validation: `cargo test`
  - Escalate if: Span coordinates drift due to trimming rules; propose an alternative coordinate space (e.g. untrimmed canvas) before continuing.
  - Started_at: 2026-02-09T11:47:01+00:00
  - Completed_at: 2026-02-09T12:18:27+00:00
  - Completion note: Added annotated Unicode render entrypoints for sequence + flowchart that return rendered text plus a stable `HighlightIndex` (`ObjectRef -> [LineSpan]`), with spans computed from renderer coordinates and clamped to trimmed output; added unit tests for one sequence + one flow fixture.
  - Validation result: `cargo test` (ok)

- [x] T003: MCP highlight tools (owner: worker:019c4239-b189-7fa2-a8ee-22fab506830e) (scope: src/mcp/) (depends: -)
  - Context: Highlights are runtime-only and must never be persisted. Tools should return deterministic applied/ignored sets.
  - DoD:
    - Add MCP tools to set/clear (and optionally get) agent-driven highlights as `ObjectRef` strings.
    - Agent highlight uses a distinct style from TUI selection highlighting; define deterministic behavior when both apply to the same region.
    - Add unit tests that set highlights, read them back, and clear them.
  - Validation: `cargo test`
  - Escalate if: Adding highlight fields breaks downstream MCP consumers; prefer new tools over changing existing response schemas.
  - Started_at: 2026-02-09T11:47:01+00:00
  - Completed_at: 2026-02-09T12:18:27+00:00
  - Completion note: Implemented MCP highlight tools `ui.set_highlights`, `ui.get_highlights`, and `ui.clear_highlights` backed by a runtime-only `agent_highlights` set; missing refs are ignored deterministically via `object_ref_is_missing`, and behavior is covered by new unit tests.
  - Validation result: `cargo test` (ok)
