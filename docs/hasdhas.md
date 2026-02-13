Biggest protocol gap (stability/persistence)

  - Protocol §7 says per-diagram .meta.json sidecars carry stable ID reconciliation for objects Mermaid can’t
    represent (notably sequence messages and flow edges).
  - The repo has DiagramMeta + SessionFolder::{load_diagram_meta, save_diagram_meta} implemented, but
    SessionFolder::save_session() never writes diagram meta, and load_session() never reads/uses it.
  - Practically: on reload, parsers re-assign IDs for sequence messages (m:0001…) and flow edges (e:0001…) from
    parse order, so old ObjectRefs (and xrefs pointing at them) can become dangling after save/load.

  Other protocol mismatches / “broken windows”

  - Protocol §2.1 lists ui_state and oplog as minimum Session fields; the model has no UI state persistence and
    no undo/redo/oplog.
  - Protocol §10 “acceptance targets” aren’t fully answerable via typed MCP tools:
      - seq.search is substring-only (no regex).
      - No direct “highest fan-in/fan-out” query for flowcharts (would require diagram.get_ast + client-side
        compute).
  - session.routes(limit=…) returns a single route; limit doesn’t produce multiple alternatives.
  - xref.list(filter) is only dangling_only today (no kind/status filtering beyond that).
  - Repo hygiene: docs/ and specs/ are gitignored (so protocol/audit aren’t versioned), and .DS_Store files
    exist while .gitignore has .DS_STORE (wrong case).

  Before I implement anything: which of these are “must-fix to call it feature complete” for you?

  - P0: diagram .meta.json write/load + ID reconciliation (to keep ObjectRefs/xrefs stable across restarts)?
  - P1: typed MCP coverage for the remaining §10 questions (regex search, fan-in/out), vs accepting
    diagram.get_ast as the escape hatch?
  - Hygiene: do you want docs//specs/ tracked in git + clean up .DS_Store?
