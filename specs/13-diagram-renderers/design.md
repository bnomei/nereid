# Design — 13-diagram-renderers

Keep diagram renderers in `src/render/` and reuse `Canvas` primitives.

Guidelines:
- renderers consume layout output (see `src/layout/`) and do not compute layout themselves beyond spacing.
- output is deterministic: stable ordering, no randomness.
- prefer a minimal baseline per diagram type first (sequence, then flowchart).

