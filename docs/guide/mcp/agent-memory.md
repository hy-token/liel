# AI memory playbook

This page describes how to use `liel[mcp]` as durable graph memory from AI tools.

## Official MCP surface

The official tool surface is fixed to these seven tools:

- `liel_overview`
- `liel_find`
- `liel_explore`
- `liel_trace`
- `liel_map`
- `liel_append`
- `liel_merge`

## Default operating pattern

Use this flow unless the project has a stronger local convention:

1. Start with `liel_overview` to understand what kind of memory already exists.
2. Use `liel_find` to narrow to relevant nodes by label or exact property.
3. Use `liel_explore` to inspect neighbourhood structure around an important node.
4. Use `liel_trace` when the question is about impact, dependency, or propagation.
5. Use `liel_map` when a Mermaid diagram will help explain the subgraph.
6. Use `liel_append` when the AI is intentionally recording new nodes and edges.
7. Use `liel_merge` when the AI wants to reuse existing nodes, update known nodes, or add idempotent edges.

## Write discipline

- Save only durable information: confirmed decisions, stable preferences, important facts, open questions worth revisiting, and tasks that should survive the session.
- Do not save chain-of-thought, speculative ideas, verbose logs, or every intermediate tool result.
- Prefer a few well-linked nodes over many tiny writes.
- Batch writes at natural checkpoints instead of committing every conversational turn.

## Stable keys for merge

`liel_merge` works best when each important label has a stable lookup key. This
is not a Claude-specific rule; it is a general AI operating rule for any MCP
client.

Good examples:

- `Module` -> `path`
- `Document` -> `url`
- `Issue` -> `issue_id`
- `Person` -> `email` or `external_id`

Recommended behavior:

- If you know a stable key, call `liel_merge` with `match`.
- If you already know the node ID, call `liel_merge` with `id`.
- If no stable key exists, use `liel_find` first and then merge by `id`.
- Avoid weak keys such as `name` alone unless uniqueness is guaranteed.

## Prompt snippets

### Claude (`CLAUDE.md`)

```md
## Project Memory
- Use `liel[mcp]` as the long-term memory store for this project when the MCP server is available.
- At the start of a task, use `liel_overview`, then `liel_find`, then `liel_explore` before asking the user to repeat known context.
- Use `liel_trace` when the user asks about impact, dependency, or what a change ripples into.
- Save only durable information: confirmed decisions, stable preferences, important facts, open questions, and tasks that should survive the session.
- Use `liel_append` for guaranteed new records and `liel_merge` when reusing existing nodes or adding idempotent edges.
- Do not save temporary reasoning, speculative ideas, or verbose logs.
```

### Codex

```md
## Durable Memory
- If the `liel` MCP server is available, use it as the default durable memory layer for the workspace.
- Restore context in this order: `liel_overview` -> `liel_find` -> `liel_explore`.
- Use `liel_trace` when estimating blast radius, ownership paths, or dependency chains.
- Save only durable project knowledge. Prefer grouped writes at meaningful checkpoints.
- Use `liel_append` when records are intentionally new. Use `liel_merge` when duplicates are possible or existing nodes should be reused.
```

### Cursor

```md
## Memory Rules
- Use `liel[mcp]` as persistent project memory when available.
- Before asking the human to restate context, check memory with `liel_overview`, `liel_find`, and `liel_explore`.
- Use `liel_map` or `liel_trace` when graph structure will clarify an answer.
- Save only durable, high-signal knowledge.
- Prefer `liel_append` for clearly new records and `liel_merge` for checkpointed updates around existing graph structure.
```
