# Canonicalization conventions

`liel` is schemaless. Canonicalization here means a set of producer-side habits
that make independently created `.liel` files easier to compare and merge. It
does not define canonical bytes, change the on-disk format, or require the Rust
core to infer semantic equality.

## What is canonical

A graph is easier to share when these parts are stable:

| Part | Convention |
|---|---|
| Node labels | Use short singular nouns in `PascalCase`, for example `Task`, `File`, `Source` |
| Edge labels | Use uppercase verb phrases, for example `MENTIONS`, `DEPENDS_ON`, `DERIVED_FROM` |
| Property names | Use `lower_snake_case` |
| Identity properties | Use explicit stable properties such as `key`, `path`, `url`, or `external_id` |
| Timestamps | Store RFC 3339 UTC strings in properties such as `created_at` and `updated_at` |
| Free text | Keep original text in `text`, `body`, or `summary`; do not hide identity in prose |

Internal node and edge IDs are not portable. They may differ after merge, pack,
vacuum, import, or export flows. If two files need to agree that a node is the
same thing, store a stable property and pass that property name to the merge
tooling.

## Identity keys

Use the narrowest stable key that really identifies the thing:

| Entity type | Common key |
|---|---|
| File | `path` |
| URL or web source | `url` |
| External issue, ticket, or object | `external_id` plus a source-specific property such as `system` |
| Project-local concept | `key` |
| Human-readable topic | `name` only when name collisions are acceptable |

Do not use timestamps, generated database IDs, or long free-text fields as merge
identity keys.

## Merge expectations

Official CLI merge behavior should stay mechanical:

- Reuse a node only when an explicit identity key says to reuse it.
- Append when no stable key is provided.
- Do not decide that two nodes are semantically equivalent.
- Do not rewrite application-specific labels into a global ontology.

This keeps local sharing deterministic and leaves domain-specific reconciliation
to application code or review tools.

## Text and paths

For text intended for comparison, producers should trim accidental surrounding
whitespace and normalize line endings to `\n`. For file paths, prefer a stable
project-relative path with `/` separators when the file is inside a project
tree. Keep absolute paths only when the host-specific location is the fact being
stored.

These are producer recommendations, not reader requirements. Readers should be
tolerant of existing files that do not follow them.

