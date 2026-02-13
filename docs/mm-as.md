# Mermaid-ASCII Feature Parity Checklist

Status reference: `mermaid-ascii` (AlexanderGrooff/mermaid-ascii) README.
Last updated: 2026-02-09.

Legend:
- **YES**: implemented end-to-end in Nereid (parse/layout/render/export as applicable)
- **PARTIAL**: some parts exist (typically parse/export-only, or render differs)
- **NO**: not implemented

Tags:
- `[C:low|mid|high]`: estimated implementation/refactor complexity in Nereid
- `[R:low|mid|high]`: relevance to Protocol 01’s goal: human+agent collaboration over a stable, queryable AST (`docs/protocol-01.md`)

## CLI / Interface (`mermaid-ascii`)

1. Standalone CLI renders to stdout (`mermaid-ascii …`) — Nereid: **YES** (via `--file` / `-f`; default is still TUI) [C:mid] [R:low]
2. Read Mermaid from file (`-f/--file`) — Nereid: **YES** [C:mid] [R:low]
3. Read Mermaid from stdin / `-f -` — Nereid: **YES** [C:mid] [R:low]
4. Adjustable horizontal spacing (`-x/--paddingX`) — Nereid: **NO** (hard-coded spacing constants) [C:mid] [R:mid]
5. Adjustable vertical spacing (`-y/--paddingY`) — Nereid: **NO** [C:mid] [R:mid]
6. Adjustable box padding (`-p/--borderPadding`) — Nereid: **NO** [C:mid] [R:mid]
7. Force ASCII-only output (`--ascii`) — Nereid: **NO** (Unicode box drawing only today) [C:mid] [R:mid]
8. Show coordinate overlay (`--coords`) — Nereid: **NO** [C:mid] [R:low]
9. Web UI/server mode (`web --port …`) — Nereid: **NO** [C:high] [R:low]
10. Shell completion command (`completion …`) — Nereid: **NO** [C:low] [R:low]

## Flowcharts / Graphs — Parsing + Export

11. Accept legacy `graph LR` header — Nereid: **NO** (we accept `flowchart …`; Protocol 01 locked decision: flowchart-only) [C:low] [R:low]
12. Accept legacy `graph TD` header — Nereid: **NO** (Protocol 01 locked decision: flowchart-only) [C:low] [R:low]
13. Direction changes layout (LR vs TD) — Nereid: **PARTIAL** (we parse `flowchart TD/LR/RL/…` but currently ignore it in layout/render) [C:high] [R:mid]
14. Labeled edges (`A -->|label| B`) — Nereid: **YES** (parse+export+render) [C:mid] [R:high]
15. Multiple arrows on one line (`A --> B --> C`) — Nereid: **NO** [C:mid] [R:low]
16. `A & B` fan-in/fan-out syntax — Nereid: **NO** [C:mid] [R:low]
17. `classDef …` syntax — Nereid: **NO** (explicitly rejected today) [C:high] [R:low]
18. `class …` syntax / `:::` class assignment — Nereid: **NO** [C:high] [R:low]
19. `subgraph … end` — Nereid: **NO** (parser currently rejects `subgraph` syntax) [C:high] [R:mid]
20. Shapes other than rectangles — Nereid: **PARTIAL** (parse+export supports round `()` + diamond `{}`; render supports rounded-corner rectangles for `round` only) [C:mid] [R:mid]
21. Whitespace + `%%` comments — Nereid: **YES** [C:low] [R:low]

## Flowcharts / Graphs — Layout + Rendering

22. Render arrowheads (directed edges look directed) — Nereid: **YES** [C:mid] [R:high]
23. Render edge labels on connectors — Nereid: **YES** [C:mid] [R:high]
24. Render subgraph boxes — Nereid: **NO** [C:high] [R:high]
25. Render non-rect node shapes visually — Nereid: **NO** [C:high] [R:mid]
26. Route edges to avoid overlapping nodes (“prevent arrows overlapping nodes”) — Nereid: **PARTIAL** (orthogonal routing with obstacles, but baseline; dense graphs can still get messy) [C:high] [R:high]
27. Diagonal arrows — Nereid: **NO** (orthogonal only) [C:high] [R:low]
28. More compact placement (tighter packing) — Nereid: **PARTIAL** (simple deterministic layout; no compaction pass) [C:high] [R:mid]
29. Clamp output width (e.g., avoid >80 cols) — Nereid: **NO** [C:mid] [R:low]
30. Colored output in terminal based on classes — Nereid: **NO** [C:high] [R:low]

## Sequence Diagrams — Parsing + Export

31. Basic messages (`A->>B: msg`) — Nereid: **YES** [C:low] [R:high]
32. Dotted return (`A-->>B: msg`) — Nereid: **YES** [C:mid] [R:mid]
33. Self-messages (`A->>A: think`) — Nereid: **YES** [C:low] [R:high]
34. Participant declarations (`participant Alice`) — Nereid: **YES** [C:low] [R:high]
35. Participant aliases (`participant A as Alice`) — Nereid: **YES** (basic: alias/label are single tokens) [C:mid] [R:high]
36. Activation syntax (`activate/deactivate`) — Nereid: **NO** [C:high] [R:low]
37. Notes (`Note left of Alice: …`) — Nereid: **NO** (model has `SequenceNote`, but no parse/export/render) [C:high] [R:high]
38. Blocks (`loop`, `alt`, `opt`, `par`, …) — Nereid: **NO** [C:high] [R:high]

## Sequence Diagrams — Rendering

39. Distinct solid vs dotted *line* rendering (`->>` vs `-->>`) — Nereid: **YES** [C:mid] [R:mid]
40. ASCII-only vs Unicode mode toggle — Nereid: **NO** [C:mid] [R:mid]
41. Unicode width-aware alignment (emoji/CJK don’t break spacing) — Nereid: **PARTIAL** (Unicode allowed, but we count `chars()` not terminal cell width) [C:mid] [R:mid]

## General

42. Support more Mermaid diagram types (class/state/ER/gantt/…) — Nereid: **NO** [C:high] [R:low]
