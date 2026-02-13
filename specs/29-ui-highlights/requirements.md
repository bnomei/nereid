# Requirements — DRAFT-29-ui-highlights

Goal: add **transient, in-memory** object highlighting in the TUI and via MCP agent commands.

Constraints:
- Highlights are **not persisted** to disk and do not survive process restart.
- Highlights are scoped to the **currently running** Nereid process/session in memory.

## Requirements (EARS)

- WHEN the TUI starts, THE SYSTEM SHALL also serve an MCP streamable-HTTP endpoint at `http://127.0.0.1:<port>/mcp`.
- WHEN the MCP server starts in TUI mode, THE SYSTEM SHALL display the bound MCP URL in the TUI so the user/agent can connect.

- WHEN an object is selected in the TUI Objects list, THE TUI SHALL visually highlight that object in the diagram pane.
- WHEN the TUI object selection changes, THE TUI SHALL update the highlight within the next render tick.
- WHEN there is no selected object, THE TUI SHALL render the diagram with no selection highlight.

- WHEN the agent calls an MCP tool to set highlights for 1+ `ObjectRef`s, THE SYSTEM SHALL highlight the referenced objects in the diagram pane.
- WHEN the agent clears highlights via MCP, THE SYSTEM SHALL remove agent-driven highlights.

- THE SYSTEM SHALL render user-driven highlights and agent-driven highlights using distinct, visually recognizable styles.
- IF a diagram region is highlighted by both user and agent simultaneously, THEN THE SYSTEM SHALL render that region with a deterministic combined style (or a deterministic precedence rule).

- IF a highlighted `ObjectRef` does not exist in the active session/diagram, THEN THE SYSTEM SHALL ignore it without crashing and SHALL return a deterministic response (e.g., applied vs ignored lists).
- THE SYSTEM SHALL NOT write highlight state to session folders or other persisted storage.
