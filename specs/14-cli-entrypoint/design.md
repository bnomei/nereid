# Design — 14-cli-entrypoint

Keep `src/main.rs` as a thin entrypoint:
- parse minimal args (std `env::args`)
- call into library modules (e.g. `nereid::tui::run()`)
- support an MCP mode (`--mcp --session <dir>`) that loads a session and serves `nereid::mcp::NereidMcp` over stdio
- map errors to `stderr` + non-zero exit
