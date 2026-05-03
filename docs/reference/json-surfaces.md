# Machine-readable surfaces index

Use this page to find the **single authoritative document** for each automation
or integration surface. Do not duplicate JSON field lists in secondary docs.

| Surface | Role | Authoritative document |
|---------|------|------------------------|
| Cross-command CLI JSON and exit codes | Automation across `diff`, `stats`, `trace`, `manifest`, `export`, `import` | [CLI JSON inventory](cli-json-inventory.md) |
| Merge preview JSON (`can_merge`, `conflicts`, `warnings`) | CI gates, MCP `liel_merge_preview` | [CLI merge report](cli-merge-report.md) |
| MCP tool payloads | Agent clients | [MCP tools reference](../guide/mcp/tools.md); merge/diff/manifest shapes defer to the CLI references above |
| Viewer and dashboard inputs | Tools that render memory without reading raw `.liel` bytes | [Viewer JSON contract](viewer-json.md) |
| External vector store hybrid | Embeddings live outside `liel` | [Vector hybrid conventions](vector-conventions.md) |
| Optional per-label validation | Team validators, not core enforcement | [Schema profiles (optional)](schema-profiles.md) |

Contributors: full ownership rules live in the source repository under
`docs/internal/process/documentation-taxonomy.ja.md` (maintainers).
