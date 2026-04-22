# Claude project memory with `liel[mcp]`

This page provides a longer `CLAUDE.md` example for projects that want Claude to use `liel[mcp]` as durable project memory without creating extra work for the human.

The main goals are:

- inspect stored memory before asking the user to repeat context
- save only durable, high-signal information
- write at meaningful checkpoints instead of every turn

## Example `CLAUDE.md`

```md
# Project Memory

## Long-term memory store
- Use `liel[mcp]` as the default long-term memory for this project when the MCP server is available.
- Treat the `.liel` database as structured project memory, not as a scratchpad.

## What to save
- Save only durable information that will help in future sessions:
  - stable user preferences and constraints
  - confirmed design decisions
  - important project facts
  - open questions worth revisiting
  - tasks that should survive the current session
- Prefer concise, high-signal memories over verbose summaries.

## What not to save
- Do not save temporary reasoning, speculative ideas, or noisy work logs.
- Do not save every intermediate tool result.
- Do not write a new memory for each small conversational turn.

## When to use liel
- At the beginning of a task, use `liel_overview`, then `liel_find`, then `liel_explore` before asking the user to repeat context.
- During investigation, use `liel_trace` when the question is about impact, dependency, or propagation.
- After a meaningful decision or discovery, save it only if it is likely to matter in a future session.
- At the end of a task or conversation, save a short durable summary if new long-term knowledge was created.

## How to write memories
- Reuse existing nodes when possible and avoid duplicate memories.
- Link new memories to related nodes so the graph stays navigable.
- Use clear labels such as `Decision`, `Task`, `Preference`, `Issue`, `Module`, or `SessionNote` when appropriate.
- Keep each memory small, explicit, and easy to query later.

## Write discipline
- Use `liel_append` when you know you are recording new graph records.
- Use `liel_merge` when existing nodes may already be present, when a known node should be updated, or when edges should be added idempotently.
- Prefer stable label-specific keys for `liel_merge` lookups, such as `path`,
  `url`, or `issue_id`.
- If no stable key exists yet, use `liel_find` first and then merge by `id`.
- Batch memory writes at natural checkpoints instead of committing every minor update.
- Favor fewer, better-connected memories over many low-value ones.
- If a memory would not help a future session, do not store it.

## Human effort
- Use liel to reduce repeated explanation from the user.
- Do not ask the user to restate known project context until you have checked the stored memory first.
- When you save something important, do it quietly and continue unless the user needs to review or correct it.
```

## Why this pattern works

- It tells Claude when to read memory, not just when to write it.
- It limits storage to durable knowledge, which keeps the graph useful over time.
- It avoids high-frequency tiny commits, which is a better fit for `liel`'s design.
- It reduces user effort by reusing stored context instead of asking for the same background repeatedly.

## Practical labels

These labels work well for many projects:

- `Decision`
- `Task`
- `Preference`
- `Issue`
- `Module`
- `SessionNote`

You do not need to define a large ontology up front. Start small and keep the graph easy to query.
