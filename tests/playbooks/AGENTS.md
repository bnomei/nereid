# Test Playbook Agents

## System:

- Human-in-the-loop evaluation system for AI behavior against the Nereid MCP server.
- Source of truth for playbook structure: this file (`AGENTS.md`).
- Scope: manual scenario execution, tool-trace validation, and answer-quality verification.

## Guardrails:

- Keep filenames as `NN_slug.md`; do not renumber existing playbooks.
- Keep scenarios deterministic and fixture-backed.
- Use exact MCP tool names and concrete object refs.
- Default to read-only; if mutating state, set `mutates_state: yes` and include reset instructions.
- Keep each playbook independent and runnable in a fresh AI conversation.

## Workflow:

1. Add playbook: choose next number, apply the format schema below, define prompt/tool expectations/output/checklist.
2. Update playbook: preserve `id` and filename, adjust only what changed in behavior or fixtures.
3. Run playbook: start Nereid/MCP per setup, submit prompt verbatim, compare trace and answer, score pass/fail.

### Playbook Format Schema

Every playbook must follow this structure so a human can run it quickly and compare AI behavior consistently.

#### 1) Header

```md
# NN - Short title
```

#### 2) Metadata

Include a compact metadata block:

```md
## Metadata
- `id`: `PB-NN`
- `goal`: one sentence
- `session`: path or mode (for example `data/demo-session`)
- `difficulty`: `basic` | `intermediate` | `advanced`
- `mutates_state`: `yes` | `no`
```

#### 3) Setup

List exact prep steps:

```md
## Setup
1. Command(s) to start Nereid and MCP.
2. Any reset requirements.
3. Preconditions that must be true before sending the prompt.
```

#### 4) User Prompt

Provide the literal prompt to send:

```md
## User Prompt
`...`
```

#### 5) Expected Tool Calls

Split calls into required, optional, and forbidden.

```md
## Expected Tool Calls
### Required (order matters)
1. `tool.name` - expected params or matcher
2. `tool.name` - expected params or matcher

### Optional (acceptable alternatives)
- `tool.name` - when/why allowed

### Forbidden
- write/mutation tools that should not be called in this scenario
```

Matching rules:

- Tool names must match exactly (for example `diagram.list`).
- Params can be exact values or constrained matchers.
- Allowed matcher keywords: `equals`, `contains`, `one_of`, `any`.
- If order matters, keep required calls in strict sequence.

#### 6) Expected Assistant Output

Describe what must appear in the AI answer:

```md
## Expected Assistant Output
- Must mention ...
- Must include ...
- Must not claim ...
```

#### 7) Pass/Fail Checklist

Keep this executable by a human:

```md
## Pass/Fail Checklist
- [ ] Required tool calls happened in order.
- [ ] No forbidden calls happened.
- [ ] Final answer satisfied output constraints.
- [ ] No hallucinated IDs, nodes, or tools.
```

#### 8) Notes

Use for tolerance rules:

```md
## Notes
- Acceptable wording variance.
- Known edge cases.
```

## Mission:

- Maintain a repeatable playbook suite that lets humans quickly verify whether an AI follows the expected MCP interaction pattern and returns correct answers.
