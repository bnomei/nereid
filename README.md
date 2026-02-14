# nereid

[![Crates.io Version](https://img.shields.io/crates/v/nereid)](https://crates.io/crates/nereid)
[![CI](https://img.shields.io/github/actions/workflow/status/bnomei/tmux-mcp/ci.yml?branch=main)](https://github.com/bnomei/tmux-mcp/actions/workflows/ci.yml)
[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json&style=flat)](https://codspeed.io/bnomei/nereid?utm_source=badge)
[![Crates.io Downloads](https://img.shields.io/crates/d/nereid)](https://crates.io/crates/nereid)
[![License](https://img.shields.io/crates/l/nereid)](https://crates.io/crates/nereid)
[![Discord](https://flat.badgen.net/badge/discord/bnomei?color=7289da&icon=discord&label)](https://discordapp.com/users/bnomei)
[![Buymecoffee](https://flat.badgen.net/badge/icon/donate?icon=buymeacoffee&color=FF813F&label)](https://www.buymeacoffee.com/bnomei)

Create and explore Mermaid diagrams in collaboration with AI agents in a terminal-first workspace, including ASCII art export.

Terminal-first diagram workspace with:
- a ratatui TUI for browsing and editing Mermaid-backed diagrams,
- an MCP server (stdio and Streamable HTTP),
- persistent session folders (diagrams, walkthroughs, xrefs, selections).

Nereid is:
- CLI-first: one binary, local-first defaults.
- MCP-first: typed tools for diagram, xref, walkthrough, and query workflows.
- Keyboard-first: fast panel navigation, hint jumps, chaining, and search.

## Installation

### From source
```bash
git clone https://github.com/bnomei/nereid.git
cd nereid
cargo build --release
```

### Cargo (local path)
```bash
cargo install --path .
```

## Quickstart

### Run TUI (default)
```bash
cargo run
```

If `nereid` is installed and on your `PATH`, run it directly in your intended session folder:

```bash
cd path/to/session
nereid .
```

By default, TUI mode also serves MCP over Streamable HTTP on:
- `http://127.0.0.1:27435/mcp`

### Run with a persisted session folder
```bash
cargo run -- path/to/session
# equivalent:
cargo run -- --session path/to/session
```

If the folder does not contain a session yet, Nereid initializes it automatically.

Persisted sessions use:
- `nereid-session.meta.json`
- `diagrams/*.mmd`
- `walkthroughs/*.wt.json`

### Demo mode
```bash
cargo run -- --demo
```

### MCP over stdio (no TUI)
```bash
cargo run -- --mcp
# with persistent session
cargo run -- --mcp --session path/to/session
```

### TUI + MCP HTTP on a custom port
```bash
cargo run -- --mcp-http-port 27500
```

## CLI

```text
nereid [<session-dir>] [--durable-writes] [--mcp-http-port <port>]
nereid [--session <dir>] [--durable-writes] [--mcp-http-port <port>]
nereid --demo [--mcp-http-port <port>]
nereid [<session-dir>] [--durable-writes] --mcp
nereid [--session <dir>] [--durable-writes] --mcp
nereid --demo --mcp
```

Notes:
- `--mcp-http-port` is only valid in TUI mode.
- `--demo` cannot be combined with `session-dir`/`--session`.
- `session-dir` and `--session` are equivalent; use one.
- `--durable-writes` enables slower best-effort fsync/sync persistence.

## MCP

Tool groups:
- `diagram.*`: `diagram.list`, `diagram.current`, `diagram.open`, `diagram.delete`,
  `diagram.create_from_mermaid`, `diagram.stat`, `diagram.get_slice`, `diagram.diff`,
  `diagram.read`, `diagram.get_ast`, `diagram.render_text`, `diagram.apply_ops`,
  `diagram.propose_ops`
- `walkthrough.*`: `walkthrough.list`, `walkthrough.open`, `walkthrough.current`,
  `walkthrough.read`, `walkthrough.stat`, `walkthrough.diff`, `walkthrough.get_node`,
  `walkthrough.render_text`, `walkthrough.apply_ops`
- `collaboration`: `attention.human.read`, `attention.agent.read`, `attention.agent.set`,
  `attention.agent.clear`, `follow_ai.read`, `follow_ai.set`, `selection.read`,
  `selection.update`, `view.read_state`
- `xref/object`: `xref.list`, `xref.neighbors`, `xref.add`, `xref.remove`, `object.read`
- `queries`: `route.find`, `seq.messages`, `seq.search`, `seq.trace`, `flow.reachable`,
  `flow.paths`, `flow.cycles`, `flow.unreachable`, `flow.dead_ends`, `flow.degrees`

Tool schemas (Input/Output):

### `diagram.get_slice`
Input:
```json
{
  "diagram_id": "d-flow",
  "center_ref": "d:d-flow/flow/node/n:a",
  "radius": 1,
  "depth": 1,
  "filters": {
    "include_categories": ["flow/node", "flow/edge"],
    "exclude_categories": []
  }
}
```
Output:
```json
{
  "objects": ["d:d-flow/flow/node/n:a", "d:d-flow/flow/node/n:b"],
  "edges": ["d:d-flow/flow/edge/e:ab"]
}
```

### `diagram.apply_ops`
Input:
```json
{
  "diagram_id": "d-seq",
  "base_rev": 3,
  "ops": []
}
```
Output:
```json
{
  "new_rev": 4,
  "applied": 1,
  "delta": { "added": [], "removed": [], "updated": [] }
}
```

### `walkthrough.apply_ops`
Input:
```json
{
  "walkthrough_id": "w:1",
  "base_rev": 0,
  "ops": []
}
```
Output:
```json
{
  "new_rev": 1,
  "applied": 1,
  "delta": { "added": [], "removed": [], "updated": [] }
}
```

### `object.read`
Input:
```json
{ "object_ref": "d:d-seq/seq/block/b:0000" }
```
Output:
```json
{
  "objects": [
    {
      "object_ref": "d:d-seq/seq/block/b:0000",
      "object": {
        "type": "seq_block",
        "kind": "alt",
        "header": "guard",
        "section_ids": ["sec:0000:00", "sec:0000:01"],
        "child_block_ids": []
      }
    }
  ],
  "context": {}
}
```

## TUI

Press `?` in-app for the full, scrollable help panel.

Common keys:
- `1` focus Diagram
- `2` toggle+focus Objects
- `3` toggle+focus XRefs
- `4` toggle Inspector
- `Tab` / `Shift-Tab` cycle focus
- `[` / `]` previous/next diagram
- `/` regular search, `\` fuzzy search, `n/N` next/previous result
- `f` hint jump, `c` chain hint mode
- `g/t` jump inbound/outbound xref
- `Space` toggle selection
- `d` deselect all objects in current diagram
- `e` edit active diagram in `$EDITOR`
- `a` toggle follow-AI attention
- `q` quit


### Theming

Nereid sticks to the terminal's ANSI palette (16 colors + text attributes like bold/dim/reverse), so it inherits your terminal theme (light/dark, base16, etc)without implementing full app theming. You can also enforce a set of colors via an `NEREID_TUI_PALETTE` environment variable.

## Configuration

Environment variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `NEREID_TUI_PALETTE` | unset | Optional palette override. |
| `NEREID_PALETTE` | unset | Alias for `NEREID_TUI_PALETTE`. |
| `VISUAL`/`EDITOR` | system | Editor used by `e` to edit Mermaid. |

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

Benchmarks:

```bash
./scripts/bench-criterion save
./scripts/bench-criterion compare
```

## License

Nereid Free Use License (No Copying, No Derivatives) v1.0. See [`LICENSE`](LICENSE).
