# Project Review: Features and Usability Gaps

## Current capabilities
- **CLI modes for TUI vs MCP + session loading.** The binary supports a default TUI mode, a `--session <dir>` mode to load an existing session folder, and a `--mcp --session <dir>` mode to run the MCP server against a persistent session directory.【F:src/main.rs†L3-L69】
- **Terminal UI with diagram viewport, sidebars, and inspector.** The TUI renders a diagram panel, an objects list, an xrefs list, and an inspector pane, with status/help text and focus highlighting for navigation.【F:src/tui/mod.rs†L47-L221】
- **Keyboard navigation for scrolling, zooming, diagram switching, selection, and xref filtering/jumps.** The TUI supports scroll/zoom in the diagram, list navigation for objects/xrefs, diagram switching via `[`/`]`, and xref filtering/jumps via `d`, `f`, `t` plus focus cycling with `Tab` and exit with `q`/Esc.【F:src/tui/mod.rs†L461-L529】
- **Session model with diagrams, walkthroughs, xrefs, and active selections.** Sessions are the top-level data container and track diagrams, walkthroughs, xrefs, and active diagram/walkthrough IDs.【F:src/model/session.rs†L8-L73】
- **Diagram types: sequence + flowchart ASTs.** Sequence diagrams include participants, messages, and notes with message kinds (sync/async/return). Flowcharts include nodes (label/shape) and edges (optional labels/styles).【F:src/model/seq_ast.rs†L5-L151】【F:src/model/flow_ast.rs†L5-L136】
- **Persistent storage with Mermaid + sidecars.** `SessionFolder` lays out session metadata, diagram `.mmd` files, ASCII exports, and walkthrough artifacts, and it can save and load sessions while parsing Mermaid syntax on load.【F:src/store/session_folder.rs†L298-L418】【F:src/store/session_folder.rs†L420-L629】
- **MCP server tools for diagrams, walkthroughs, xrefs, and rendering.** The MCP server exposes tools like `session.list_diagrams`, `walkthrough.list`, `xref.add`, `diagram.render_text`, and `diagram.apply_ops`, and serves over stdio for integration.【F:src/mcp/server.rs†L111-L134】【F:src/mcp/server.rs†L290-L311】【F:src/mcp/server.rs†L983-L1025】【F:src/mcp/server.rs†L2319-L2465】

## Usability gaps / issues
- **TUI is explicitly an early “app shell” with static rendering + basic scrolling.** The top-level UI comment notes this is an app shell with a static diagram buffer and basic scrolling, which signals limited interactivity and early-stage usability for day-to-day work.【F:src/tui/mod.rs†L20-L22】
- **No in-TUI editing or persistence workflows.** The TUI key handling only supports navigation, zoom, selection, filtering, and diagram switching; there are no edit/insert/remove commands or save actions in the UI loop.【F:src/tui/mod.rs†L461-L529】
  *Usability impact:* users can inspect diagrams/xrefs but cannot author or update them directly in the terminal UI.
- **Zoom is a character-scaling trick, not a re-layout.** `scale_diagram` repeats characters to approximate zoom, which can distort diagrams rather than reflowing layout or text intelligently.【F:src/tui/mod.rs†L715-L738】
  *Usability impact:* zoom may reduce legibility or introduce visual artifacts for dense diagrams.
- **TUI doesn’t expose walkthroughs, despite model + MCP support.** Walkthroughs exist in the session model and MCP tools, but the TUI panes only list objects/xrefs + diagram view and inspector, with no walkthrough navigation or rendering surface.【F:src/model/session.rs†L8-L73】【F:src/mcp/server.rs†L290-L311】【F:src/tui/mod.rs†L47-L221】
- **CLI is intentionally minimal.** The CLI only supports `--session` and `--mcp` flags and defaults to the demo TUI; there’s no visible CLI command to initialize or scaffold a new session folder from scratch.【F:src/main.rs†L3-L69】

## Suggestions (prioritized usability improvements)
1. **Add TUI editing workflows** (add/edit/delete nodes/edges/messages, update labels, save session) to move beyond read-only exploration.【F:src/tui/mod.rs†L461-L529】【F:src/store/session_folder.rs†L420-L629】
2. **Expose walkthroughs in the TUI** (list, navigate, and render walkthroughs, possibly linking to diagram objects). The model and MCP already support this, so adding UI surface would close a feature gap.【F:src/model/session.rs†L8-L73】【F:src/mcp/server.rs†L290-L311】【F:src/tui/mod.rs†L47-L221】
3. **Improve zoom behavior** by re-rendering at the layout level rather than character scaling to keep diagrams readable at different magnifications.【F:src/tui/mod.rs†L715-L738】
4. **Enhance CLI ergonomics** with subcommands (e.g., `init`, `load`, `render`, `export`) to make sessions easier to create/manage and to provide non-TUI workflows.【F:src/main.rs†L3-L69】
