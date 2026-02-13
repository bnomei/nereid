# Design — 02-dependencies

Own all edits to `Cargo.toml` (and by extension `Cargo.lock`) in one place.

Dependency groups we expect to need:
- Serialization for session persistence (likely `serde` + JSON backend).
- TUI stack (`ratatui` + terminal backend, likely `crossterm`).
- MCP server implementation (use `rmcp` v0.14.0; tool surface is captured in spec task `Context:` blocks, not by pointing workers at `docs/protocol-01.md`).
- MCP runtime support (direct `tokio` dependency for a stdio server entrypoint) and schemas (direct `schemars` for `JsonSchema` derives used by rmcp tool parameter/output types).
- Performance toolbox (added ahead of perf refactors to reduce manifest churn):
  - micro-opts:
    - `itoa`: fast integer → decimal string (avoid `format!`/`Formatter` overhead in tight loops)
    - `memchr`: SIMD-accelerated byte search (delimiters/newlines) for parse/export helpers
    - `smallvec`: inline small vectors (avoid heap allocs for the common “few items” case)
    - `smol_str`: small-string storage with cheap clones for many short IDs/tokens (use only if bench-gated)
  - parallelism:
    - `rayon`: data-parallel iterators/join; use selectively with thresholds and stable ordering for determinism
  - profiling (dev-only):
    - `pprof`: Criterion profiler integration to emit flamegraphs (see `scripts/bench-criterion.md`)

We will introduce dependencies “just in time” to avoid blocking std-only foundational work.

## Decisions

- TUI backend: `ratatui` + `crossterm` (common pairing; cross-platform; supports raw mode + key events).
- MCP SDK: `rmcp` v0.14.0.
- Perf helper crates are present, but usage is still bench-gated by the perf specs (if it doesn’t improve, discard).
