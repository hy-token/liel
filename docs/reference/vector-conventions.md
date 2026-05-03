# Vector hybrid conventions

`liel` keeps **vectors and ANN search out of the Rust core** by design (see
[product trade-offs](../design/product-tradeoffs.md)). Teams that use an
external vector database should store **references** on nodes as ordinary
properties and keep embeddings in the external system.

## Recommended properties (optional)

Pick stable names your tools agree on; these are **conventions**, not enforced
schema:

| Property | Meaning |
|----------|---------|
| `embedding_model` | Model id or version string used for the stored reference |
| `embedding_ref` | Opaque id or URI in the vector store for this node (or chunk) |
| `embedding_space` | Logical namespace / collection name in the provider |

For chunked documents, either reference multiple external ids or use separate
nodes per chunk linked by edges.

## Operational rules

1. **Single writer** — Respect `liel`’s single-writer guard; update vector refs
   in the same workflow that updates graph structure when possible.
2. **No automatic sync** — The core does not push/pull embeddings; your jobs
   reconcile graph and vector store.
3. **Privacy** — Treat `embedding_ref` like any other sensitive property if it
   leaks retrieval scope.

## Related

- [Schema profiles (optional)](schema-profiles.md) — stronger optional contracts
- [Machine-readable surfaces index](json-surfaces.md)
