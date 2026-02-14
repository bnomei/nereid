# Nereid MCP Playbooks

Use these payloads with the `nereid` skill. Keep calls small and local first.
Treat session files (`nereid-session.meta.json`, `diagrams/*.mmd`, `walkthroughs/*.wt.json`) as app-managed state snapshots that may be rewritten frequently.
Use MCP/TUI tools so session metadata and revision history stay coherent.

## Startup and target resolution

```bash
# TUI + MCP HTTP
cargo run -- --session path/to/session

# stdio MCP
cargo run -- --mcp --session path/to/session
```

Resolve active diagram:
1. `diagram.current`
2. if null: `diagram.list`
3. then: `diagram.open`

Resolve active walkthrough with the same pattern:
1. `walkthrough.current`
2. if null: `walkthrough.list`
3. then: `walkthrough.open`

If you need a new diagram first, call `diagram.create_from_mermaid`:

```json
{
  "mermaid": "flowchart TD\n  A --> B",
  "diagram_id": "d-my-flow",
  "name": "My Flow",
  "make_active": true
}
```

Creation runs parse + render preflight. If the call returns `INVALID_PARAMS` with
`cannot render Mermaid diagram: ...` (for example cycle/layout issues), fix Mermaid and retry
before proceeding.
For create/switch-only tasks, use the create response as the success signal; skip
`diagram.stat`/`diagram.render_text`/`flow.*` unless inspection is explicitly requested.
Failed preflight is non-mutating: no new diagram is persisted and active diagram remains unchanged.

Delete a diagram when cleaning up:

```json
{
  "diagram_id": "d-my-flow"
}
```

## Live collaboration state

### Read human attention

Tool: `attention.human.read`

```json
{}
```

### Set and read agent attention

Tool: `attention.agent.set`

```json
{
  "object_ref": "d:d-auth-flow/flow/node/n:authorize"
}
```

Tool: `attention.agent.read`

```json
{}
```

### Clear agent attention

Tool: `attention.agent.clear`

```json
{}
```

### Follow-AI mode

Tool: `follow_ai.read`

```json
{}
```

Tool: `follow_ai.set`

```json
{
  "enabled": true
}
```

### Shared working set

Tool: `selection.update`

```json
{
  "object_refs": [
    "d:d-auth-flow/flow/node/n:start",
    "d:d-auth-flow/flow/node/n:authorize"
  ],
  "mode": "replace"
}
```

## Probe before edit

Tool: `diagram.stat`

```json
{
  "diagram_id": "d-auth-flow"
}
```

Tool: `diagram.get_slice`

```json
{
  "diagram_id": "d-auth-flow",
  "center_ref": "d:d-auth-flow/flow/node/n:start",
  "radius": 2
}
```

Canonical object ref format:
`d:<diagram_id>/<seq|flow>/<participant|message|node|edge>/<object_id>`

## Safe mutation pattern

1. `diagram.propose_ops`
2. if result is good: same payload to `diagram.apply_ops`

```json
{
  "diagram_id": "d-auth-flow",
  "base_rev": 3,
  "ops": [
    {
      "type": "flow_add_node",
      "node_id": "n:authorize",
      "label": "Authorize",
      "shape": "rect"
    },
    {
      "type": "flow_add_edge",
      "edge_id": "e:authorize",
      "from_node_id": "n:start",
      "to_node_id": "n:authorize",
      "label": "token ok"
    }
  ]
}
```

## Sequence insertion

```json
{
  "diagram_id": "d-checkout-seq",
  "base_rev": 7,
  "ops": [
    {
      "type": "seq_add_participant",
      "participant_id": "p:fraud",
      "mermaid_name": "FraudService"
    },
    {
      "type": "seq_add_message",
      "message_id": "m:fraud-check",
      "from_participant_id": "p:api",
      "to_participant_id": "p:fraud",
      "kind": "sync",
      "text": "validate(payment)",
      "order_key": 35
    }
  ]
}
```

## Cross-diagram mapping and routes

Tool: `xref.add`

```json
{
  "xref_id": "x:authorize-impl",
  "from": "d:d-auth-flow/flow/node/n:authorize",
  "to": "d:d-checkout-seq/seq/message/m:fraud-check",
  "kind": "implements",
  "label": "authorization path"
}
```

Tool: `xref.list` (dangling TODOs)

```json
{
  "dangling_only": true
}
```

Tool: `route.find`

```json
{
  "from_ref": "d:d-auth-flow/flow/node/n:start",
  "to_ref": "d:d-checkout-seq/seq/message/m:fraud-check",
  "limit": 3,
  "max_hops": 12,
  "ordering": "fewest_hops"
}
```

## Walkthrough refinement

Tool: `walkthrough.apply_ops`

```json
{
  "walkthrough_id": "wt:auth-overview",
  "base_rev": 2,
  "ops": [
    {
      "type": "add_node",
      "node_id": "wn:entry",
      "title": "Entry path",
      "body_md": "Request enters API and reaches auth gate.",
      "refs": [
        "d:d-auth-flow/flow/node/n:start"
      ],
      "tags": [
        "overview"
      ],
      "status": "ok"
    },
    {
      "type": "add_edge",
      "from_node_id": "wn:entry",
      "to_node_id": "wn:authorize",
      "kind": "next",
      "label": "auth step"
    }
  ]
}
```

## Conflict handling

On stale `base_rev`:
1. refresh with `diagram.diff` or `walkthrough.diff`,
2. rebase ops,
3. retry apply.

If diff history is unavailable, fetch `diagram.read` or `walkthrough.read` once, then return to diff/slice-first calls.
