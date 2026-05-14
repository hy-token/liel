# Claude project-memory sample

This example points Claude at a single `.liel` file through the `liel` MCP
server. It is a workflow sample, not a full agent runtime.

## Quick path

1. Install the MCP extra:

   ```bash
   pip install "liel[mcp]"
   ```

2. Configure Claude to launch `liel-mcp` with an explicit project memory path:

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

3. Copy or adapt the sample project instructions from
   `docs/guide/mcp/samples/CLAUDE.md`.

4. Ask Claude to restore context with `liel_overview`, `liel_find`, and
   `liel_explore` before asking you to repeat project history.

5. Review the resulting memory outside Claude:

   ```bash
   liel stats .liel/project-memory.liel --format json
   liel export .liel/project-memory.liel -o target/project-memory.export.json
   liel manifest .liel/project-memory.liel -o target/project-memory.manifest.json
   ```

See `docs/guide/mcp/claude-workflow.md` for the full walkthrough.
