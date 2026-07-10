# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - 2026-07-10
- Introduced a shared graph/track render pipeline: domain ASTs lower into `GraphModel` / `TrackModel`, then family layout and paint, so new diagram kinds add lowerers instead of private layout/render stacks.
- Rehomed flowchart and sequence diagram rendering through that pipeline with a temporary domain-AST bridge; existing Unicode snapshots stay bit-identical.
- Added graph compartment painting (variable-height multi-compartment boxes with full-width interior dividers) and an expanded single-cell endpoint cap alphabet for class/ER-style relations.
- Added class diagrams end-to-end: `ClassAst`, limited Mermaid `classDiagram` parse/export, graph-family lowerer, compartment render path, and session/MCP/TUI kind wiring (`classDiagram` create/replace/export/render).
- Added entity-relationship diagrams: `ErAst`, limited `erDiagram` parse/export with 2→1 char cardinality folding, graph lowerer, and session/MCP/TUI kind wiring.
- Added gantt charts: `GanttAst`, limited `gantt` parse/export (sections, dates, durations, `after`), track bar paint, and session/MCP/TUI kind wiring.
- Expanded the built-in demo session with class, ER, and gantt fixtures linked from the demo index.
- Added top-level node notes for class and ER diagrams (AST + diagram sidecar + render/TUI), matching flowchart notes.
- Polished gantt paint toward sequence-style track boxes: drop title/section chrome, put time labels on top, and draw multi-column content boxes (shared with sequence 1-lane headers) that can hold notes.
- Gantt time lanes use sequence-style header boxes + lifelines; F/C hints work for `gantt/task` and `gantt/lane` (class/er/gantt notes dim via `*/note` highlight spans).

## [0.8.0] - 2026-07-09
- Added sequence structure ops for alt/opt/loop/par blocks, sections, and message membership (including nested ancestor membership).
- Added `diagram_replace_from_mermaid` with identity reconciliation (messages, blocks/sections, participants/nodes/edges) and dangling-xref reporting scoped to the target diagram.
- Persisted sequence block/section ids in diagram sidecars so reloads and replaces keep custom structure refs when fingerprints match.
- Hardened MCP/ops deltas and replace history so `diagram_diff` and structure consumers stay consistent after membership moves, message prune, and in-memory replace.

## [0.7.0] - 2026-07-07
- Added a Vim/Helix-style `:` diagram switcher in the TUI with fuzzy matching, `Tab` completion, and `[SEQ]`/`[FLO]` match rows.
- Surfaced the diagram switcher in the TUI footer/help and README key reference.
- Added typed Frigg symbol anchors for sequence participants and flow nodes, including MCP reads, set/clear ops, and sidecar persistence without changing Mermaid source.

## [0.6.0] - 2026-07-07
- Renamed MCP tool names from dotted namespaces to underscore names for CallMcpTool bridge compatibility.
- Regenerated the MCP tool schema snapshot and updated Nereid playbooks, skill docs, and README tool references.

## [0.5.0] - 2026-06-28
- Hardened persisted session writes against concurrent updates, stale lock files, and failed meta commits.
- Made MCP diagram and walkthrough updates more consistent when disk-backed sessions change concurrently.
- Improved TUI pending diagram sync metadata normalization for selections and cross-reference statuses.
- Fixed Mermaid sequence participant redeclarations and message pruning inside branch/nested blocks.

## [0.4.0] - 2026-06-17
- Improved manual CLI parser help, version output, and actionable parse errors.
- Added a reviewable MCP tool schema snapshot and snapshot freshness test.
- Split the MCP server implementation into tool-group modules.
- Refreshed README metadata badges, installation links, and license wording.
- Clarified source-available noncommercial license terms in package metadata and release scripts.

## [0.3.0] - 2026-05-09
- Added the dedicated Notes pane.
- Updated Cargo dependencies.

## [0.2.0] - 2026-03-23
- Upgraded the MCP server stack to `rmcp` 1.2
- Refreshed direct dependency patch/minor releases in the lockfile

## [0.1.2] - 2026-02-26
- Vastly improved diagram rendering quality with a new pipeline
- Added hint (f/c keys) for edges in flow diagrams

## [0.1.0] - 2026-02-12
- Initial Release
