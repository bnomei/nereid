# Design — DRAFT-29-ui-highlights

## Overview

We want object highlighting driven by:
1) TUI selection (Objects list)
2) MCP calls from an agent

Highlight state is **runtime-only** and must not be persisted.

## Data model (runtime-only)

Add a small UI runtime state (separate from `Session` so it never touches persistence):

- `UiRuntimeState`
  - `selected: Option<ObjectRef>` (from TUI Objects selection)
  - `agent_highlights: BTreeSet<ObjectRef>` (set/cleared via MCP)

When rendering, the effective highlight set is:
- `effective = agent_highlights ∪ selected(if any)`

Render `selected` (user-driven) with a distinct style from `agent_highlights` (agent-driven).
If the same region is highlighted by both, render a deterministic “both” style (or apply a
deterministic precedence rule).

## Rendering strategy (annotated render)

We need a stable way to highlight *the right characters* in the rendered diagram, not just substring-matching labels.

Approach:
- Add annotated render entrypoints that return:
  - the rendered diagram text (same as today)
  - a `HighlightIndex`: `BTreeMap<ObjectRef, Vec<LineSpan>>`

Where `LineSpan` is `(y, x0, x1)` in **character cell coordinates** (inclusive), relative to the returned text lines.

### Sequence diagrams

Compute highlight spans deterministically from the same coordinate math used by the renderer:
- Participant highlight:
  - participant header box rectangle
  - (optional) lifeline column from header to bottom
- Message highlight:
  - arrow segment on the message row (including head)
  - (optional) message label region

### Flowcharts

Compute highlight spans from the same metrics as the flowchart renderer:
- Node highlight:
  - node box rectangle
- Edge highlight:
  - routed connector polyline segments (including stubs to boxes)

### Converting spans into ratatui text

In the TUI, build a `ratatui::text::Text` from rendered lines by:
- For each line, slice by character indices into alternating `Span`s with either normal or highlight `Style`.
- Clamp spans to the line length to avoid panics if the renderer trims trailing whitespace.

Highlight styles:
- `user` (TUI selection): yellow background + black foreground + bold.
- `agent` (MCP-driven): cyan background + black foreground + bold.
- `both` (overlap): magenta background + black foreground + bold.

## MCP interface

Add MCP tools (names tentative):
- `ui.set_highlights({ object_refs: [string], mode: "replace"|"add"|"remove" }) -> { applied: [string], ignored: [string] }`
- `ui.clear_highlights() -> { cleared: u64 }`
- (optional) `ui.get_highlights() -> { object_refs: [string] }`

These mutate/read `UiRuntimeState.agent_highlights`.

## TUI + MCP in the same process (needed for live agent-driven highlighting)

The interactive TUI should start an MCP server **at the same time** so an agent can connect
to `http://127.0.0.1:<port>/mcp` using RMCP streamable HTTP.

Implementation sketch:
- Use RMCP `StreamableHttpService` (feature: `transport-streamable-http-server`) mounted at `/mcp`
  in an `axum::Router` via `nest_service("/mcp", service)`.
- Bind a `tokio::net::TcpListener` on `127.0.0.1:<port>` (support `port=0` for ephemeral).
- Run `axum::serve(listener, router)` concurrently with the TUI loop.
- Display the final bound URL in the TUI status bar (e.g. `MCP: http://127.0.0.1:41237/mcp`).

Concurrency model (minimal-change option):
- Run the HTTP server on a Tokio runtime task.
- Run the TUI on a blocking thread (`spawn_blocking`) so it can keep using synchronous crossterm APIs.
- Use a shared `Arc<tokio::sync::Mutex<SharedState>>`:
  - MCP handlers `await` the mutex.
  - The TUI uses `blocking_lock()` to read highlight state (and, if desired, session state) without
    rewriting the entire TUI as async.

## Persistence

Do not modify `SessionFolder` persistence format. The highlight state lives only in `UiRuntimeState`.

## Open decisions

- Port selection UX (fixed default vs `0` ephemeral) and how the agent discovers the chosen port.
