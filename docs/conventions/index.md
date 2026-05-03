# Conventions

Entry point for `.liel` sharing conventions. These are conventions, not schema,
and they are not enforced by the Rust core.

## Read in this order

1. [Recommended labels](recommended-labels.md) - starter vocabulary plus naming/normalization basics  
2. [Provenance conventions](provenance.md) - source tracking workflow

## Optional (advanced)

- [Vector hybrid conventions](../reference/vector-conventions.md) - referencing external embeddings alongside `liel` nodes (not enforced by the core)  
- [Schema profiles (optional)](../reference/schema-profiles.md) - per-label expectations for validators and team linting

`canonicalization.md` is kept for compatibility and now points to the merged
rules in `recommended-labels.md`.

## Responsibilities in one minute

- `recommended-labels.md`: what names and keys to store
- `provenance.md`: how to preserve evidence and derivation traceability

## Minimal operating rules

1. Keep label/property naming consistent  
2. Use `url`/`path` or `system + external_id` as identity  
3. Use `Source` and `DERIVED_FROM` for traceability  

