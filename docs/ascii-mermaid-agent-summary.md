# ASCII-Only Mermaid Diagramming with Agent Collaboration (Summary)

## Goal

Build a **local-only**, **ASCII-only** TUI application in **Rust** that lets a human and an LLM agent collaboratively create, edit, and reason about **Mermaid flowcharts and sequence diagrams**.

Key properties:
- No browser, no SVG, no network calls
- ASCII / Unicode rendering only
- Interactive TUI using `ratatui`
- Agent connected via MCP, operating on structured data (AST / graph)
- Diagram is both **renderable** and **queryable**

---

## Core Architecture

```
Mermaid-like DSL
      ↓
   Parser
      ↓
Diagram AST / Graph  ←───────┐
      ↓                       │
 Layout Engine                │
      ↓                       │
 ASCII Renderer → ratatui UI  │
                              │
                    MCP Tools │
                              │
                         LLM Agent
```

The **AST is the source of truth**:
- Rendering reads from it
- Agent reasoning queries it
- Editing mutates it

---

## Diagram Scope (Initial)

### Supported Types
- **Flowcharts**
- **Sequence diagrams**

Why:
- Both map cleanly to graphs
- Both already proven in ASCII (see references)
- Sequence diagrams are ideal for “reasoning” use‑cases

Explicitly out of scope (initially):
- Gantt, ER, C4, mindmaps, styling, themes

---

## ASCII Rendering Strategy

### Rendering Model
- Fixed-width grid (rows × columns)
- Boxes, arrows, lifelines built from primitives:
  - `+ - |`
  - Unicode box drawing (`┌ ┐ └ ┘ ─ │ ▶ ◀`)
- No font assumptions beyond monospace

### Inspiration Repos
- **mermaid-ascii** (Go): proven layout + routing logic
- **beautiful-mermaid** (TS): AST → layout → ASCII pipeline
- **ADia**: text-first sequence diagrams
- **svgbob**: parsing ASCII → graph (inverse problem, still useful)

---

## Layout & Theory

### Flowcharts
- Directed acyclic graph (mostly)
- Use **Sugiyama-style layered layout**:
  - Assign layers
  - Minimize crossings
  - Route edges orthogonally

### Sequence Diagrams
- Essentially **ordered trees with time axis**
- Columns = participants
- Rows = messages
- Lifelines = vertical rails
- Messages = horizontal edges

Sequence diagrams are **not force-directed**.
They are deterministic and timeline-based.

Trees are a good mental model for:
- Call hierarchies
- Nested interactions
- Agent explanation (“this call happens inside that loop”)

---

## AST Design (Key Part)

### Flowchart AST (Example)
```rust
Node { id, label, shape }
Edge { from, to, label }
Graph { nodes, edges }
```

### Sequence Diagram AST (Example)
```rust
Participant { id, name }
Message {
  from,
  to,
  text,
  index,
  kind // sync, async, return
}
Sequence { participants, messages }
```

This AST enables:
- Rendering
- Validation
- Querying
- Regeneration

---

## “Ask the Graph” (Concept Clarification)

There is **no single official “Ask Graph” product**.

It is a **pattern**:
> Use structured graph / AST data as the authority,
> let the LLM reason *over it* instead of guessing.

Examples:
- “Who sends the first message?”
- “Does A ever call C?”
- “What happens after step 5?”

Implementation options:
- Let LLM inspect AST directly (small graphs)
- Or expose **query tools** via MCP

This is the same idea used in:
- Knowledge graph QA
- Code AST analysis
- Static analysis tools

---

## Known MCP Tool Design

Your MCP server can expose tools like:

- `diagram.get_ast()`
- `diagram.add_message(from, to, text)`
- `diagram.remove_node(id)`
- `diagram.query(question)`
- `diagram.explain(element_id)`

The LLM:
- Never edits ASCII directly
- Only mutates the AST via tools
- ASCII is a **projection**, not the data

---

## Agent + TUI Collaboration

### Human
- Navigates diagram in TUI
- Types natural language or commands
- Selects nodes / messages

### Agent
- Suggests edits
- Applies changes via MCP tools
- Answers questions using AST
- Explains structure (“this branch causes…”)

This avoids:
- Prompt hallucinations
- ASCII drift
- Inconsistent state

---

## Debugging & Instrumentation

Helpful features:
- Toggle AST view (JSON / tree)
- Highlight selected node in ASCII
- Show coordinates / grid overlay
- Step-by-step layout mode
- “Why is this edge here?” explanations

You can expose debug tools only to the agent.

---

## ratatui Responsibilities

- Window layout
- Key handling
- Scroll
- Panel separation:
  - Diagram
  - Logs
  - Agent chat
  - Inspector

Rendering logic stays separate from UI logic.

---

## Why This Is Interesting for Agents

Mermaid diagrams become:
- **Stateful artifacts**
- **Queryable knowledge**
- **Regenerable**
- **Explainable**

Not just pictures.

This unlocks:
- Interactive system design reviews
- Live architecture reasoning
- Teaching tools
- Agent-driven documentation

---

## Next Steps

1. Build minimal parser → AST for sequence diagrams
2. Deterministic ASCII renderer
3. ratatui wrapper
4. MCP server with 3–5 core tools
5. Agent reasoning loop

From there:
- Versioning
- Diffing
- Time travel
- Multi-agent critique

---

## Key References

- ratatui (Rust TUI)
- mermaid-ascii (Go)
- beautiful-mermaid (TS)
- svgbob (Rust)
- Sugiyama layered graph layout
- MCP (Model Context Protocol)

---

This document is intended as **Codex CLI research input** and architectural guidance.
