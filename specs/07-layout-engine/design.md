# Design — 07-layout-engine

Keep layout algorithms in `src/layout/`.

Sequence layout:
- columns = participants
- rows = ordered messages / blocks
- deterministic spacing configuration later

Flowchart layout:
- layered layout (Sugiyama-style) for DAG-first
- orthogonal edge routing baseline first; improve iteratively

