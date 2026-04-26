# Design (philosophy and trade-offs)

This section freezes what **liel** aims for as a portable external brain for LLM workflows and, just as importantly, what it deliberately does not do.
The byte-level format and reliability contract live under [Behavior and specifications](../reference/index.md), while API-level operational guidance lives next to the relevant interfaces in the guide.

| Document | Content |
|---|---|
| [Design principles](principles.md) | Short statement of the product's core values and how they map to the rest of the design docs |
| [Architecture overview](architecture.md) | Logical model, page and WAL structure, adjacency lists, and system layering |
| [Single-writer guard](single-writer-guard.md) | How `liel` rejects unsafe double-writer opens and reclaims stale lock directories |
| [Product trade-offs](product-tradeoffs.md) | Scope, deliberate non-goals, concurrency stance, and storage-format decisions |
