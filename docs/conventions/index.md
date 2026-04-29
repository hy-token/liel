# Conventions

These pages describe recommended graph conventions for sharing and aggregating
`.liel` files.

The conventions are not a schema and are not enforced by the Rust core. They
exist above the storage engine so teams and local agents can create files that
are easier to diff, merge, pack, and review.

| Document | Read when you need to |
|---|---|
| [Canonicalization conventions](canonicalization.md) | Choose stable labels, property names, and identity keys before sharing files |
| [Recommended labels](recommended-labels.md) | Start with a small vocabulary for memory, sources, tasks, and project work |

## Principles

- Keep the `.liel` file format unchanged.
- Treat database IDs as local implementation details, not portable identity.
- Prefer explicit properties and edges over hidden meaning in free text.
- Use conventions only when they make files easier to exchange or inspect.
- Let applications extend the vocabulary for their own domain.

