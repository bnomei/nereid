# nereid

[![Crates.io Version](https://img.shields.io/crates/v/nereid)](https://crates.io/crates/nereid)
[![Crates.io Downloads](https://img.shields.io/crates/d/nereid)](https://crates.io/crates/nereid)
[![CI](https://img.shields.io/github/actions/workflow/status/bnomei/nereid/ci.yml?branch=main&label=CI)](https://github.com/bnomei/nereid/actions/workflows/ci.yml)
[![CodSpeed](https://img.shields.io/github/actions/workflow/status/bnomei/nereid/codspeed.yml?branch=main&label=CodSpeed)](https://github.com/bnomei/nereid/actions/workflows/codspeed.yml)
[![License](https://img.shields.io/badge/license-source--available%20noncommercial-blue)](LICENSE)
[![Discord](https://flat.badgen.net/badge/discord/bnomei?color=7289da&icon=discord&label)](https://discordapp.com/users/bnomei)
[![Buymecoffee](https://flat.badgen.net/badge/icon/donate?icon=buymeacoffee&color=FF813F&label)](https://www.buymeacoffee.com/bnomei)

Nereid is a terminal-first workspace for Mermaid-backed diagrams. It combines a `ratatui` TUI, a Model Context Protocol (MCP) server, folder-based session persistence, and deterministic text renders so humans and AI agents can inspect and update the same diagram session.

Use Nereid when you want to:

- browse and edit sequence diagrams and flowcharts from a terminal,
- keep diagrams, walkthroughs, cross-references, selections, and active state in a local session folder,
- expose typed MCP tools for diagram navigation, mutation, xrefs, walkthroughs, and graph queries,
- export Mermaid sources plus text previews for reviewable, file-based workflows.

Nereid is source-available for noncommercial use only. Commercial use, product use, paid service use, internal business use, redistribution, sublicensing, or modified distribution requires separate written permission. See [License](#license).

<a title="click to open" target="_blank" style="cursor: zoom-in;" href="https://raw.githubusercontent.com/bnomei/nereid/main/screenshot.png"><img src="https://raw.githubusercontent.com/bnomei/nereid/main/screenshot.png" alt="Nereid TUI screenshot" style="width: 100%;" /></a>

## Installation

### Prerequisites

- Rust 1.85 or newer when installing with Cargo or building from source.
- A terminal that supports alternate-screen TUI applications.
- Optional: `VISUAL` or `EDITOR` for the in-app Mermaid editor. Nereid falls back to `vi`.

Installing Nereid does not grant unrestricted rights. The same source-available noncommercial license applies to crates.io, Homebrew, GitHub Releases, and source builds.

### Cargo

```bash
cargo install nereid
nereid --version
```

### Homebrew

```bash
brew install bnomei/nereid/nereid
nereid --version
```

### GitHub Releases

Download a release archive from [GitHub Releases](https://github.com/bnomei/nereid/releases), review the included license notice, extract the archive, and put the `nereid` binary on your `PATH`.

### From source

```bash
git clone https://github.com/bnomei/nereid.git
cd nereid
cargo build --release
./target/release/nereid --version
```

## Quickstart

Run the built-in demo session:

```bash
nereid --demo
```

From a source checkout, use Cargo:

```bash
cargo run -- --demo
```

Expected result:

- The TUI opens with demo diagrams.
- Press `?` to open the in-app help.
- Press `q` to quit.
- While the TUI runs, MCP is available over Streamable HTTP at `http://127.0.0.1:27435/mcp`.

Create or open a persistent session folder:

```bash
mkdir my-session
nereid my-session
```

If the folder has no session metadata and no existing `diagrams/*.mmd` files, Nereid initializes a seed `flow` diagram. For an existing session, Nereid loads `nereid-session.meta.json` and the referenced diagram and walkthrough files.

## Run modes

| Task | Command | Result |
| --- | --- | --- |
| Open the current directory as a TUI session | `nereid` | Loads or initializes `.` and starts MCP HTTP on port `27435`. |
| Open a specific session folder | `nereid path/to/session` | Loads or initializes that folder. |
| Use an explicit session flag | `nereid --session path/to/session` | Same as a positional session directory. |
| Run the demo session | `nereid --demo` | Starts the TUI with a temporary demo session. |
| Change the TUI MCP HTTP port | `nereid --mcp-http-port 27500` | Serves MCP at `http://127.0.0.1:27500/mcp`. |
| Run MCP over stdio without the TUI | `nereid --mcp --session path/to/session` | Serves MCP on stdin/stdout for tool integrations. |
| Print the MCP schema snapshot | `nereid --dump-mcp-tool-schema` | Prints the registered tool schemas and exits. |

CLI synopsis:

```text
nereid [<session-dir>] [--durable-writes] [--mcp-http-port <port>]
nereid [--session <dir>] [--durable-writes] [--mcp-http-port <port>]
nereid --demo [--mcp-http-port <port>]
nereid [<session-dir>] [--durable-writes] --mcp
nereid [--session <dir>] [--durable-writes] --mcp
nereid --demo --mcp
nereid --dump-mcp-tool-schema
```

Rules:

- If `session-dir` and `--session` are omitted, Nereid uses the current working directory.
- `--demo` cannot be combined with a session directory.
- `--mcp-http-port` is only valid in TUI mode.
- `--durable-writes` opts into slower best-effort durable persistence with fsync or sync where supported.

Source: [src/main.rs](src/main.rs).

## Session folders

A session folder is the source of truth for durable Nereid state. The TUI and persistent MCP server both read and write this folder.

| Path | Purpose |
| --- | --- |
| `nereid-session.meta.json` | Session id, active diagram and walkthrough ids, diagram index, xrefs, and selection state. |
| `diagrams/*.mmd` | Canonical Mermaid source for each diagram. |
| `diagrams/*.meta.json` | Stable id maps and extracted metadata for persisted diagrams. |
| `diagrams/*.ascii.txt` | Best-effort text render export for each diagram. The extension is legacy; contents may include Unicode box drawing. |
| `walkthroughs/*.wt.json` | Walkthrough graph data. |
| `walkthroughs/*.ascii.txt` | Best-effort text render export for each walkthrough. |
| `.nereid-session.write.lock` | Transient write lock used during session saves. |

Nereid may rewrite managed files while it runs. Use the TUI `e` key to edit the active diagram in `$VISUAL` or `$EDITOR`, or stop Nereid before making manual changes. Text render exports are generated asynchronously and can briefly lag behind `.mmd` or `.wt.json` updates during rapid edits.

Source: [src/store/session_folder.rs](src/store/session_folder.rs).

## Mermaid support

Nereid parses a deliberate Mermaid subset and reports unsupported syntax as an actionable error.

Sequence diagrams support:

- `sequenceDiagram` as the first non-empty line,
- comments beginning with `%%`,
- `participant <name>` and `<role> <name>` declarations,
- message lines such as `alice->>bob: Hello`,
- Mermaid message arrows normalized internally to sync, async, or return messages,
- `alt`, `opt`, `loop`, and `par` blocks,
- `else` inside `alt`, `and` inside `par`, and `end` block closures.

Flowcharts support:

- `flowchart` or `graph` as the first non-empty line, with optional `TD`, `TB`, `LR`, `RL`, or `BT` direction,
- comments beginning with `%%`,
- node declarations: `<id>`, `<id>[<label>]`, `<id>(<label>)`, and `<id>{<label>}`,
- edges with common Mermaid flowchart operators, normalized internally,
- edge labels in `A -->|label| B` and `A -- label --> B` forms,
- chained edges such as `A --> B --> C`,
- `linkStyle` statements, which are preserved on export but ignored by the text renderer.

Identifiers in parsed Mermaid node and participant names must be non-empty ASCII alphanumeric strings or underscores. They cannot contain whitespace or `/`.

Sources: [src/format/mermaid/sequence.rs](src/format/mermaid/sequence.rs), [src/format/mermaid/flowchart.rs](src/format/mermaid/flowchart.rs), [src/format/mermaid/ident.rs](src/format/mermaid/ident.rs).

## TUI

Press `?` in Nereid for the full scrollable help panel.

Common keys:

| Key | Action |
| --- | --- |
| `q` | Quit. |
| `1` | Focus the diagram pane. |
| `2` / `3` | Toggle and focus Objects / XRefs. |
| `4` / `5` | Toggle Inspector / Notes. |
| `Tab` / `Shift-Tab` | Focus next / previous panel. |
| `[` / `]` | Open previous / next diagram. |
| `:` | Open the diagram switcher. |
| `/` / `\` | Regular / fuzzy search. |
| `n` / `N` | Next / previous search result. |
| Arrow keys or `h`/`j`/`k`/`l` | Pan or move within the focused panel. |
| `f` | Hint jump mode. |
| `c` | Chain hint selection mode. |
| `Space` | Toggle selected object. |
| `d` | Deselect all objects in the current diagram. |
| `y` | Yank the selected object ref with OSC52. |
| `g` / `t` | Jump through inbound / outbound xrefs. |
| `e` | Edit the active diagram in `$VISUAL`, `$EDITOR`, or `vi`. |
| `a` | Toggle follow-AI attention. |

### Palette override

By default, Nereid uses the terminal ANSI palette. Set `NEREID_TUI_PALETTE` to override foreground, background, and the 16 ANSI colors.

Value shape:

```text
<fg>,<bg>,<black>,<red>,<green>,<yellow>,<blue>,<magenta>,<cyan>,<white>,<bright_black>,<bright_red>,<bright_green>,<bright_yellow>,<bright_blue>,<bright_magenta>,<bright_cyan>,<bright_white>
```

Each color must be `#RRGGBB`, `0xRRGGBB`, or `rgb:rr/gg/bb`. `NEREID_PALETTE` is accepted as an alias.

Sources: [src/tui/chrome.rs](src/tui/chrome.rs), [src/tui/theme.rs](src/tui/theme.rs).

## MCP

Nereid exposes MCP tools for live diagram collaboration.

Transports:

- TUI mode: Streamable HTTP at `http://127.0.0.1:<port>/mcp`, default port `27435`.
- Stdio mode: `nereid --mcp --session path/to/session`.

Tool groups:

- Diagram lifecycle and reads: `diagram_list`, `diagram_current`, `diagram_open`, `diagram_delete`, `diagram_read`, `diagram_stat`, `diagram_get_ast`, `diagram_get_slice`, `diagram_render_text`, `diagram_diff`.
- Diagram creation and mutation: `diagram_create_from_mermaid`, `diagram_propose_ops`, `diagram_apply_ops`.
- Walkthroughs: `walkthrough_list`, `walkthrough_current`, `walkthrough_open`, `walkthrough_read`, `walkthrough_stat`, `walkthrough_get_node`, `walkthrough_render_text`, `walkthrough_diff`, `walkthrough_apply_ops`.
- Collaboration state: `attention_human_read`, `attention_agent_read`, `attention_agent_set`, `attention_agent_clear`, `follow_ai_read`, `follow_ai_set`, `selection_read`, `selection_update`, `view_read_state`.
- Cross-references and objects: `xref_list`, `xref_neighbors`, `xref_add`, `xref_remove`, `object_read`.
- Queries: `route_find`, `seq_messages`, `seq_search`, `seq_trace`, `flow_reachable`, `flow_paths`, `flow_cycles`, `flow_unreachable`, `flow_dead_ends`, `flow_degrees`.

Object refs use this canonical shape:

```text
d:<diagram_id>/<category...>/<object_id>
```

Examples:

```text
d:demo-flow/flow/node/n:a
d:demo-seq/seq/message/m:0001
```

The reviewable schema snapshot lives at [src/mcp/server/tool_schema.snapshot.json](src/mcp/server/tool_schema.snapshot.json). Regenerate it after changing tool names, descriptions, input schemas, or output schemas:

```bash
cargo run -- --dump-mcp-tool-schema > src/mcp/server/tool_schema.snapshot.json
cargo test mcp_tool_schema_snapshot_is_current
```

Sources: [src/mcp/server.rs](src/mcp/server.rs), [src/mcp/server/tool_schema.snapshot.json](src/mcp/server/tool_schema.snapshot.json), [src/model/object_ref.rs](src/model/object_ref.rs).

## Demo data and playbooks

The repository includes a persisted demo session in [data/demo-session](data/demo-session). It contains sequence diagrams, flowcharts, xrefs, and a walkthrough that exercise the TUI and MCP tools.

Playbook prompts for MCP clients live in [tests/playbooks](tests/playbooks). Example tasks include:

- "From the demo index, list every nav xref that lands in a sequence diagram."
- "In the flowchart demo with alpha/beta edges, return the local neighborhood around node A."
- "Create a temporary flowchart diagram, delete it, and confirm it is gone from `diagram_list`."

## Development

Run the standard checks before review:

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

Run Criterion benchmark baselines:

```bash
./scripts/bench-criterion save
./scripts/bench-criterion compare
```

Run local pre-commit hooks:

```bash
prek validate-config prek.toml
prek run --all-files
prek install
```

Release helpers live in [scripts](scripts), and GitHub Actions workflows live in [.github/workflows](.github/workflows).

## License

Nereid Source-Available Noncommercial License v1.0. Noncommercial use is allowed only under the terms in [LICENSE](LICENSE). Commercial use, product use, paid service use, internal business use, redistribution, sublicensing, modified distribution, or incorporation into another project requires separate prior written permission from the copyright holder.
