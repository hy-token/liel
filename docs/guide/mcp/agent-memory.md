# AI memory playbook

This page is the recommended operating pattern for using `liel[mcp]` as durable
graph memory from AI tools.

## Which file should I use?

| File | Purpose |
|---|---|
| This page | Recommended operating pattern for any LLM |
| [Claude setup](claude.md) | Claude-specific setup pointer |
| [Claude project-memory workflow](claude-workflow.md) | End-to-end setup → record → trace → review sample |
| [Sample `CLAUDE.md`](samples/CLAUDE.md) | Copyable Claude project-instructions sample |

If you only add one rule to an agent prompt, use this:

```md
Always check existing memory before asking the user to repeat context.
```

## Start small

For most projects, begin with this policy:

- Save only durable, high-signal information: decisions, preferences, tasks,
  sources, and important project facts.
- Do not save temporary reasoning, speculative notes, noisy logs, or every tool result.
- Read existing memory at the start of a task.
- Write at meaningful checkpoints, not every turn.
- Use nodes for entities and edges for relationships.

Useful starter labels:

- `Task`
- `Decision`
- `Preference`
- `Issue`
- `Module`
- `Source`

## Official MCP surface

For `1.0`, read / inspection tools are stable candidates and write tools remain
experimental. Treat `liel_append` and `liel_merge` as project-policy tools: use
them only when the project has a clear stable-key convention, dedupe rule, and
human review path for durable memory writes.

The official tool surface is fixed to these ten tools:

- `liel_overview`
- `liel_find`
- `liel_explore`
- `liel_trace`
- `liel_map`
- `liel_diff`
- `liel_merge_preview`
- `liel_manifest`
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

Because MCP mutation tools are experimental for `1.0`, make write policy explicit
before enabling autonomous writes. At minimum, define which labels may be written,
which properties are stable keys, when to reuse existing nodes, and when a human
must review the diff / trace before the memory becomes release evidence.

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

## Project write policy template (copy and edit)

Use this block as a starting `MEMORY_WRITE_POLICY.md` (or an equivalent section in
`AGENTS.md`) **before** enabling autonomous MCP writes. Replace placeholders and
delete rows you do not need.

```md
# Project memory write policy

## Allowed labels (MCP append / merge)

| Label | Stable key property | Notes |
|-------|---------------------|-------|
| Task | `title` + `status` | Example row — replace with your vocabulary |
| Decision | `path` or `decision_id` | |
| Source | `url` | |

## When to use `liel_append`

- The record is intentionally new and must not dedupe against existing nodes.
- No stable key exists yet (capture first, normalize keys in a later human-reviewed pass).

## When to use `liel_merge`

- A stable key is known and uniqueness is enforced by project convention.
- Idempotent edges or updates to nodes that already exist in memory.

## Dedupe rules

- Same stable key on the same label → one canonical node; incoming duplicates are merged per `--on-node-conflict` policy after human review when policy is ambiguous.

## Human review required

- Any change that alters a `Decision` node’s meaning or reverses a recorded preference.
- Bulk imports from agent sessions before the file is promoted to release evidence.
- First merge from a new machine or untrusted export path.

## Before adding to release evidence

1. `liel stats <file>.liel --format json` — counts sane.
2. `liel manifest` / `verify` if the project signs memory.
3. `liel merge --dry-run --fail-on-conflict` when identity rules apply.
4. Spot-check `liel trace` for impacted `Decision` / `Task` nodes.
```

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
- Use the repository-root `codex-session-memory.liel` as the canonical project memory file for this repo (see `AGENTS.md`). Do not substitute another `.liel` as the default unless the user says so.
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
