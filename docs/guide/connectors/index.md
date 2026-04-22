# APIs and integrations

Interfaces to the liel core and local graph memory. Today the public surfaces are the Python binding (PyO3) and the optional MCP server.

---

## Available

| Surface | Install | Status |
|---|---|---|
| **[Python API](python.md)** | `pip install liel` | Stable |
| **[MCP server](../mcp/index.md)** | `pip install "liel[mcp]"` | Stable |

## Planned

Other-language bindings (Node.js, Go, ...) depend on a future **C FFI** (Phase 3 candidate). Today PyO3 wraps the `GraphDB` in `src/db.rs` directly; the **[format spec](../../reference/format-spec.md)** is the canonical reference for any third-party reader/writer.

**Expectation alignment:** the semantics of the graph API and the recommended write patterns are described in **[product trade-offs](../../design/product-tradeoffs.md)**. Read it alongside this integration documentation.
