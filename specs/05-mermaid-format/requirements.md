# Requirements — 05-mermaid-format

This spec implements parsing and canonical export for the supported Mermaid subsets:
- `sequenceDiagram`
- modern `flowchart` (not legacy `graph`)

Normative protocol reference: `docs/protocol-01.md`

## Requirements (EARS)

- THE SYSTEM SHALL parse supported Mermaid subset syntax into the AST.
- THE SYSTEM SHALL export the AST back to canonical `.mmd` (formatting may change).
- THE SYSTEM SHALL reject unsupported syntax with actionable errors.

