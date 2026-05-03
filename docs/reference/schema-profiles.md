# Schema profiles (optional)

The Rust core remains **schemaless**: any labels and properties are allowed.
Teams that want **predictable memory** can define optional **profiles** outside
the database: JSON documents that describe expected properties per label, used by
validators, linters, or CI checks—not by `liel` itself.

## Relationship to other mechanisms

| Mechanism | Purpose |
|-----------|---------|
| **Identity rules** (`liel diff --identity-rules`, `liel merge --identity-rules`) | Stable identity for diff/merge across files — see the [command-line guide](../guide/cli.md) |
| **Schema profile (this page)** | Document “what we usually store” for humans and scripts |
| **Core enforcement** | Not planned for Phase 4; rejected changes stay in validators |

## Profile shape (informative)

A profile file might list, per label, recommended or required property keys and
value types. There is **no single blessed filename** in the core repo; teams may
reuse the same JSON structure as identity rules or maintain a separate file.

Example (illustrative only):

```json
{
  "Task": {
    "required": ["system", "external_id"],
    "optional": ["status", "embedding_ref"]
  }
}
```

Validators read `.liel` via Python or CLI exports, then compare records against
the profile.

## Related

- [Vector hybrid conventions](vector-conventions.md)
- [Recommended labels](../conventions/recommended-labels.md)
- [Machine-readable surfaces index](json-surfaces.md)
