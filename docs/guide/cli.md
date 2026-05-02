# Command line

The `liel` console script is the shared command-line entry point for local file
operations. It is implemented in Python and calls the public Python package API.
The Rust core and `.liel` file format are unchanged.

Existing specialized scripts remain available:

| Script | Purpose |
|---|---|
| `liel` | General local file operations |
| `liel-demo` | Bundled agent-memory demo |
| `liel-mcp` | Optional MCP server |

## Smoke files

From a source checkout, generate small ignored `.liel` files for manual CLI
checks:

```bash
python examples/09_cli_smoke_files.py --force
```

The script reads fixed CSV data from `examples/cli_smoke_data/`, writes `.liel`
files under `target/cli-smoke/`, and prints copyable `diff` and `merge`
commands. The CSV files are tracked; the generated `.liel` files are not.

On Windows, if a manual CLI smoke run leaves generated `.liel.lock` directories
behind, clean only this smoke directory with:

```bash
python examples/09_cli_smoke_files.py --clean-locks
```

## Help

```bash
liel help
liel help diff
liel help merge
liel help pack
liel help manifest
liel help sign
liel help verify
liel help stats
liel help export
liel help import
```

`liel help` prints top-level help. `liel help <command>` prints help for a
specific command. The usual `--help` and `-h` flags are also available.

## Version

```bash
liel version
liel version --format json
```

## Diff

```bash
liel diff left.liel right.liel
liel diff left.liel right.liel --format json
liel diff left.liel right.liel --node-key path
liel diff left.liel right.liel --identity-rules identity-rules.json
```

`liel diff` is read-only. It compares live node and edge records mechanically by
their current IDs and properties unless an explicit identity rule is provided.
It does not infer that two nodes are semantically equivalent.

### ID-based comparison

The current diff identity rule is local ID matching:

```text
left node 1  <->  right node 1
left edge 7  <->  right edge 7
```

This rule is intended for comparing the same file lineage, such as a file before
and after edits. It is not a cross-file identity rule for independently created
`.liel` files.

Under this rule:

- a record ID present only on the right is `added`
- a record ID present only on the left is `removed`
- a record ID present on both sides but with different normalized record content
  is `changed`
- node labels are compared in sorted order
- node and edge properties are compared by exact stored values
- edge endpoints are compared by their current local node IDs

### Key-aware comparison

For independently created `.liel` files, use `--node-key` when every node has a
stable property that identifies the same real-world object across files:

```bash
liel diff left.liel right.liel --node-key path
liel diff left.liel right.liel --node-key system --node-key external_id
```

The first Phase 3 key-aware mode is intentionally strict:

- every node in both files must have all requested key properties
- the resulting key must be unique inside each file
- ambiguous or missing keys fail closed with exit code `2`
- matched nodes are compared without considering their local IDs
- edges are compared as a multiset of key-resolved endpoint, label, and
  property signatures
- without a future `--edge-key`, edge property changes are reported as one
  removed edge plus one added edge

Fuzzy matching, duplicate candidates, and partial review reports are left for
later conflict / candidate diff work. `--node-key` only confirms differences
when explicit stable keys make the identity rule unambiguous.

When different labels need different identity properties, pass an identity rules
JSON file instead of repeating one key for every node:

```bash
liel diff left.liel right.liel --identity-rules identity-rules.json
```

```json
{
  "identity_rules": {
    "File": ["path"],
    "Task": ["system", "external_id"]
  }
}
```

The initial identity-rules mode is also strict:

- every node must match exactly one rule by label
- the rule's properties must be present on that node
- the resulting identity must be unique inside each file
- nodes with no matching rule, multiple matching labels, missing keys, or
  duplicate identities fail closed with exit code `2`
- edges are compared as a multiset of resolved endpoint identities, edge label,
  and edge properties, so duplicate parallel edges are preserved by count

Exit codes:

| Code | Meaning |
|---:|---|
| `0` | The files have no mechanical differences |
| `1` | Differences were found |
| `2` | Usage error, such as a missing input file |

## Merge

```bash
liel merge base.liel incoming.liel -o merged.liel
liel merge base.liel incoming.liel --dry-run --format json
```

`liel merge` copies the base file to the output path, then merges the incoming
file into that output with `GraphDB.merge_from`. It never writes into either
input file.

By default, merge is append-oriented:

```bash
liel merge base.liel incoming.liel -o merged.liel
```

Use `--dry-run` to preview the merge report without writing an output file:

```bash
liel merge base.liel incoming.liel --dry-run --format json
liel merge base.liel incoming.liel -o merged.liel --dry-run --format json
```

When `--dry-run` is set, `-o/--output` is optional and is treated only as the
planned output path in the report. The command copies the base file to a
temporary preview file, runs the merge there, returns the same counters and ID
maps as a real merge, and deletes the preview file.

For key-aware dry runs, the JSON report includes `can_merge` and `conflicts`.
Missing source keys, duplicate source keys, and ambiguous destination matches
are reported as conflicts instead of creating an output file.

When a key-aware dry run can merge but would discard or overwrite node property
values according to `--on-node-conflict`, the JSON report includes `warnings`.
Property warnings do not change `can_merge`; they are review material for values
that the selected merge policy will ignore or replace. Label differences on
reused nodes are also reported as warnings because merge keeps destination
labels.

When files share a stable property, pass that property as a node identity key:

```bash
liel merge base.liel incoming.liel -o merged.liel --node-key key
liel merge base.liel incoming.liel -o merged.liel --node-key system --node-key external_id
liel merge base.liel incoming.liel -o merged.liel --identity-rules identity-rules.json
```

`--identity-rules` accepts the same JSON shape as `liel diff`. It lets merge use
different identity properties per label, such as `File.path` and
`Task.system + Task.external_id`. It is mutually exclusive with `--node-key`.

For identity-rules merge, source nodes must match exactly one rule. Destination
nodes that match a rule must also be unambiguous; destination nodes with no
matching rule are ignored as reuse candidates. Dry-run reports rule problems as
`conflicts` with `can_merge: false`.

Useful options:

| Option | Meaning |
|---|---|
| `--node-key NAME` | Reuse nodes by property name. Repeat for a compound key |
| `--identity-rules PATH` | Reuse nodes with label-specific identity rules from JSON |
| `--edge-strategy append` | Always append merged edges. This is the default |
| `--edge-strategy idempotent` | Reuse an exactly matching merged edge when possible |
| `--on-node-conflict keep_dst` | Keep existing destination properties on key collision |
| `--on-node-conflict overwrite_from_src` | Replace reused node properties with source properties |
| `--on-node-conflict merge_props` | Fill missing destination properties from source |
| `--dry-run` | Preview the merge report without writing an output file |
| `--force` | Allow overwriting the output path |
| `--format json` | Emit a machine-readable merge report |

The output path must be different from both input files. In-place merge is not a
supported command-line operation.

## Pack

```bash
liel pack source.liel packed.liel --include-labels Person,Task
liel pack source.liel packed.liel --include-labels Person --include-labels Task
liel pack source.liel packed.liel --include-labels Person --format json
```

`liel pack` extracts nodes with the selected labels into a new `.liel` file. It
also copies only edges whose endpoints are both included, remapping node IDs in
the output file.

Useful options:

| Option | Meaning |
|---|---|
| `--include-labels LABELS` | Comma-separated node labels to include. Repeat to add more labels |
| `--force` | Allow overwriting the output path |
| `--format json` | Emit a machine-readable pack report |

The output path must be different from the input file. In-place pack is not a
supported command-line operation.

## Manifest

```bash
liel manifest graph.liel
liel manifest graph.liel -o graph.liel.manifest.json
```

`liel manifest` emits deterministic JSON for review, Git storage, and future
signature verification. The manifest is generated from the `.liel` contents and
does not include the input file name, local path, or generation time.

The initial manifest rules are intentionally narrow:

- UTF-8 JSON with LF line endings and one trailing newline
- JSON object keys are sorted
- nodes and edges are sorted by local ID
- node labels are sorted
- properties are represented under sorted JSON keys
- file names, absolute paths, and timestamps from the local run are excluded

Useful options:

| Option | Meaning |
|---|---|
| `-o, --output PATH` | Write the manifest JSON to a file instead of stdout |
| `--force` | Allow overwriting the output path |

## Sign And Verify

```bash
liel sign graph.liel --key-file secret.key -o graph.liel.sig
liel verify graph.liel --key-file secret.key --signature graph.liel.sig
liel verify graph.liel --key-file secret.key --signature graph.liel.sig --format json
```

`liel sign` signs the deterministic manifest bytes for a `.liel` file and
writes an external signature JSON file. It does not write into the `.liel` file.

The initial signature mode uses `hmac-sha256` from the Python standard library.
The key file bytes are used exactly as stored. This is a shared-secret
integrity check, not a public-key signature scheme. Future signature versions
can add public-key algorithms without changing the `.liel` file format.

`liel verify` regenerates the manifest from the current `.liel` file and checks
it against the external signature. It exits with `0` when the signature matches
and `1` when the current file, signature, or key do not match.

Useful `sign` options:

| Option | Meaning |
|---|---|
| `--key-file PATH` | File containing the HMAC key bytes |
| `-o, --output PATH` | Write the signature JSON to a file instead of stdout |
| `--force` | Allow overwriting the output signature path |

Useful `verify` options:

| Option | Meaning |
|---|---|
| `--key-file PATH` | File containing the HMAC key bytes |
| `--signature PATH` | Signature JSON file to verify |
| `--format json` | Emit a machine-readable verify report |

## Stats

```bash
liel stats graph.liel
liel stats graph.liel --format json
```

`liel stats` prints a compact summary of a `.liel` file. It is useful before
reviewing an unfamiliar file and after `pack`, `merge`, `import`, or `export`
workflows to confirm counts and label composition.

The report includes:

- file format version
- file size
- node and edge counts
- node label counts
- edge label counts

## Export And Import

```bash
liel export graph.liel -o graph.json
liel import graph.json -o restored.liel
liel import graph.json -o restored.liel --format json
```

`liel export` writes a deterministic JSON reconstruction format. The initial
export format is JSON only; there is no `--as` or data-format option.

`liel import` reads export JSON and creates a new `.liel` file. It does not
expect the input arrays to be sorted. Source IDs from the JSON are treated as
reference IDs: import sorts nodes and edges by source ID, creates fresh `.liel`
records, and remaps edge endpoints through the resulting ID map. Import
preserves relationships, but it does not promise that imported `.liel` IDs will
always match source JSON IDs.

Useful options:

| Option | Meaning |
|---|---|
| `-o, --output PATH` | Write the export JSON or imported `.liel` file |
| `--force` | Allow overwriting the output path |
| `--format json` | Emit a machine-readable import report |

### Manifest vs export

`liel manifest` and `liel export` intentionally have separate JSON contracts.

The manifest contract is the stable verification contract. A
`manifest_version` must not receive breaking changes because manifest bytes are
used by `liel sign` and `liel verify`.

The export contract is the reconstruction contract. It is deterministic and
Git-friendly, but it may evolve to support import workflows. Breaking import or
export changes must use a new `export_version`.

## Conventions

For files that will be shared or merged repeatedly, use the
[canonicalization conventions](../conventions/canonicalization.md) and
[recommended labels](../conventions/recommended-labels.md). They are not
required, but stable labels and explicit identity properties make command-line
diff and merge results easier to review.
