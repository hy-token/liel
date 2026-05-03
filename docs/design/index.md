# Design (philosophy and trade-offs)

This section freezes what **liel** aims for as a portable external brain for LLM workflows and, just as importantly, what it deliberately does not do.
Beyond storage, the design assumes memory files may be **shared, diffed, and merged** like other team artifacts—reviewability is part of the product boundary described here and in [Product trade-offs](product-tradeoffs.md).
The byte-level format and reliability contract live under [Behavior and specifications](../reference/index.md), while API-level operational guidance lives next to the relevant interfaces in the guide.

| Document | Content |
|---|---|
| [Design principles](principles.md) | Short statement of the product's core values and how they map to the rest of the design docs |
| [Architecture overview](architecture.md) | Logical model, page and WAL structure, adjacency lists, and system layering |
| [Single-writer guard](single-writer-guard.md) | How `liel` rejects unsafe double-writer opens and reclaims stale lock directories |
| [Product trade-offs](product-tradeoffs.md) | Scope, deliberate non-goals, concurrency stance, and storage-format decisions |

### Maintainer paths (full repository)

The published MkDocs site excludes `docs/internal/**`. For Wave D **CodingMemory** and **Focused Memory API** design sources (Japanese), browse the repository on GitHub:

| Topic | Path |
|-------|------|
| CodingMemory use-case scope | [`docs/internal/design/coding-memory.ja.md`](https://github.com/hy-token/liel/blob/main/docs/internal/design/coding-memory.ja.md) |
| Focused Memory API design notes | [`docs/internal/design/memory-api.ja.md`](https://github.com/hy-token/liel/blob/main/docs/internal/design/memory-api.ja.md) |
| Phase 4 Git-manageable positioning | [`docs/internal/design/phase4-git-manageable-agent-memory.ja.md`](https://github.com/hy-token/liel/blob/main/docs/internal/design/phase4-git-manageable-agent-memory.ja.md) |
| LangGraph / LangChain boundary (substrate, JA) | [`docs/internal/design/langgraph-liel.ja.md`](https://github.com/hy-token/liel/blob/main/docs/internal/design/langgraph-liel.ja.md) |
| CodingMemory helper (Python, experimental) | [`python/liel/coding_memory.py`](https://github.com/hy-token/liel/blob/main/python/liel/coding_memory.py) |

Authoritative doc-role index: [`docs/internal/process/documentation-taxonomy.ja.md`](https://github.com/hy-token/liel/blob/main/docs/internal/process/documentation-taxonomy.ja.md) §7.
