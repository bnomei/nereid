# Design — 03-model-core

Keep model types in `src/model/` only.

Design constraints:
- std-only for this spec (no new dependencies).
- The model must align with the protocol types in `docs/protocol-01.md`.
- Keep “semantics” in the model; keep rendering/layout/query logic out of the model.

Implementation notes (non-normative):
- Prefer one file per concept (session, diagram, xref, walkthrough, seq ast, flow ast) if it keeps code readable.

