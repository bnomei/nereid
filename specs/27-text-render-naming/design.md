# Design — 27-text-render-naming

## Current state

- MCP exposes `diagram.render_ascii` and `walkthrough.render_ascii`, but both are implemented via `render_*_unicode` renderers.
- Session folder exports deterministic text renders to `*.ascii.txt`, but those files also contain Unicode.

This is intentional for now (Unicode renderers exist), but the naming is confusing and leaks into variable names and documentation.

## Proposed approach (compat-first)

### Terminology

Adopt “text” as the user-facing term:
- “text render”: deterministic text diagram; Unicode allowed.
- “ascii render”: reserved for future 7-bit ASCII-only output (not implemented here).

### MCP tool surface

Add new MCP tools with accurate naming:
- `diagram.render_text({ diagram_id? }) -> { text }`
- `walkthrough.render_text({ walkthrough_id }) -> { text }`

Backwards compatibility:
- Keep existing tools `diagram.render_ascii` and `walkthrough.render_ascii` as aliases that return the same content (and optionally mark them as deprecated in their descriptions).

### Session folder exports

Two viable export strategies:

1. **Keep writing `*.ascii.txt`** for backwards compatibility, but rename internal variables and comments to “text/unicode”.
2. Write **both** `*.ascii.txt` (legacy) and `*.text.txt` (new), with docs steering consumers to the new name.

Decision: **(1) keep writing `*.ascii.txt` only** for now.

Backwards-compat notes:
- `diagram.render_ascii` / `walkthrough.render_ascii` remain supported and return the same content as the new `*.render_text` tools.
- Session exports keep the legacy filename `*.ascii.txt` to avoid changing on-disk expectations; docs and code comments will treat “ascii” as a legacy label for deterministic text (Unicode allowed).

## Validation

- `cargo test --offline`
- MCP tests: ensure `render_text` matches `render_ascii`.
- Store tests: if a new filename is introduced, add a test asserting both contents match; if not, at least update docs/comments to avoid “ASCII” claims.
