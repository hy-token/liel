# Claude setup

This page is only about using the MCP server from Claude. For the general
memory rules that apply to any AI tool, start with the
[AI memory playbook](agent-memory.md).

## Which file should I use?

| File | Purpose |
|---|---|
| [AI memory playbook](agent-memory.md) | Recommended operating pattern for any LLM |
| [Sample `CLAUDE.md`](samples/CLAUDE.md) | Copyable Claude project-instructions sample |
| [Claude project-memory workflow](claude-workflow.md) | End-to-end setup → record → trace → review sample |
| This page | Claude-specific setup pointer |

## Add the memory policy

After registering the `liel` MCP server with Claude, add a project memory
policy to your Claude project instructions.

For a ready-to-adapt sample, see [sample `CLAUDE.md`](samples/CLAUDE.md).

The most important rule is:

```md
Always check existing memory before asking the user to repeat context.
```

## Why this matters

Claude will only use a memory tool well if the project instructions define both
read discipline and write discipline:

- read memory before asking the user to repeat context
- save only durable, high-signal information
- write at meaningful checkpoints, not every turn
- use nodes for entities and edges for relationships

Those rules are explained in the [AI memory playbook](agent-memory.md).

## End-to-end workflow

After the basic setup, follow the [Claude project-memory workflow](claude-workflow.md) for a reproducible setup → memory creation → record → trace → review path.
