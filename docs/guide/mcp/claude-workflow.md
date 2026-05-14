# Claude project-memory workflow

This is the first end-to-end ecosystem sample for `liel`: use Claude with the
`liel` MCP server as durable, reviewable project memory.

The sample is intentionally small. It does **not** implement an agent runtime,
a hosted memory service, or automatic semantic memory extraction. Claude remains
the user-facing assistant; `liel` provides the local graph memory file and the
read / write / inspect tools.

## What this workflow demonstrates

1. Start Claude with a single `.liel` file as project memory.
2. Read existing memory before asking the human to repeat context.
3. Record a durable project decision with stable labels and links.
4. Trace impact paths through the memory graph.
5. Review the memory as a portable artifact using CLI JSON and the sample viewer.

## 1. Install the MCP extra

```bash
pip install "liel[mcp]"
```

For local development from this repository, use the normal maintainer setup from
`requirements-dev.txt` and then run `maturin develop` before trying the MCP
server.

## 2. Choose the project memory file

Use one explicit file per project. For this example:

```bash
mkdir -p .liel
liel stats .liel/project-memory.liel --format json
```

If the file does not exist yet, the MCP server can create it when it starts with
`--path`. `stats` is useful after creation because it proves the file opens and
emits machine-readable counts.

## 3. Register the MCP server with Claude

Use the installed `liel-mcp` executable and an absolute memory path:

```json
{
  "mcpServers": {
    "liel": {
      "type": "stdio",
      "command": "/absolute/path/to/liel-mcp",
      "args": ["--path", "/absolute/path/to/project/.liel/project-memory.liel"]
    }
  }
}
```

Do not put `mcpServers` in `.claude/settings.json`; use the MCP configuration
mechanism for your Claude environment.

## 4. Add Claude project instructions

Copy the memory policy from the [sample `CLAUDE.md`](samples/CLAUDE.md), or add
this minimum rule:

```md
Always check existing memory before asking the user to repeat context.
```

Recommended Claude behavior:

- Start tasks with `liel_overview`, then `liel_find`, then `liel_explore`.
- Use `liel_trace` for impact, dependency, or propagation questions.
- Save only durable information: decisions, preferences, important facts, open
  questions, and tasks that should survive the session.
- Use `liel_append` for clearly new records.
- Use `liel_merge` when existing nodes may already exist or when edges should be
  idempotent.
- Do not save chain-of-thought, speculative ideas, verbose logs, or every tool
  result.

## 5. Example session shape

Ask Claude to follow this project-memory discipline during a coding task:

```text
We are deciding where to store sessions for the auth migration.
Check project memory first. If there is no existing decision, record the final
choice with a Task, OpenQuestion, Decision, and Source node, then link the
Decision to the implementation file once we pick it.
```

Expected MCP flow:

1. `liel_overview` — confirm what labels and counts already exist.
2. `liel_find` — search for existing auth/session decisions.
3. `liel_explore` — inspect neighbouring context if a related node exists.
4. `liel_merge` — record or update durable decision records using stable keys.
5. `liel_trace` — explain how the task, decision, source, and implementation
   file are connected.

## 6. Review the memory outside Claude

The memory remains a normal local artifact. Review it with CLI commands:

```bash
liel stats .liel/project-memory.liel --format json
liel export .liel/project-memory.liel -o target/project-memory.export.json
liel manifest .liel/project-memory.liel -o target/project-memory.manifest.json
```

Open the export in the read-only sample viewer:

```text
docs/guide/sample-viewer/app/index.html
```

For review of branch changes, preview a key-aware merge before writing:

```bash
liel merge base.liel incoming.liel \
  --dry-run --fail-on-conflict --format json --node-key path
```

## Responsibility boundary

`liel` owns:

- the local `.liel` memory file;
- graph records and relationships;
- MCP tools for reading, writing, tracing, diffing, merging, and manifesting;
- reviewable CLI JSON outputs.

Claude owns:

- the conversation;
- deciding when a memory is worth saving;
- summarising user intent;
- applying the project instructions.

The human owns:

- project policy;
- correcting bad memories;
- deciding what becomes a release gate or release evidence.
