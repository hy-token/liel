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
```

`liel diff` is read-only. It compares live node and edge records mechanically by
their current IDs and properties. It does not infer that two nodes are
semantically equivalent and it does not apply a global schema.

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

Future key-aware and manifest-aware diff modes should reuse the same identity
resolution layer, then change only how records are paired before reporting
`added`, `removed`, and `changed`.

Exit codes:

| Code | Meaning |
|---:|---|
| `0` | The files have no mechanical differences |
| `1` | Differences were found |
| `2` | Usage error, such as a missing input file |

## Merge

```bash
liel merge base.liel incoming.liel -o merged.liel
```

`liel merge` copies the base file to the output path, then merges the incoming
file into that output with `GraphDB.merge_from`. It never writes into either
input file.

By default, merge is append-oriented:

```bash
liel merge base.liel incoming.liel -o merged.liel
```

When files share a stable property, pass that property as a node identity key:

```bash
liel merge base.liel incoming.liel -o merged.liel --node-key key
liel merge base.liel incoming.liel -o merged.liel --node-key system --node-key external_id
```

Useful options:

| Option | Meaning |
|---|---|
| `--node-key NAME` | Reuse nodes by property name. Repeat for a compound key |
| `--edge-strategy append` | Always append merged edges. This is the default |
| `--edge-strategy idempotent` | Reuse an exactly matching merged edge when possible |
| `--on-node-conflict keep_dst` | Keep existing destination properties on key collision |
| `--on-node-conflict overwrite_from_src` | Replace reused node properties with source properties |
| `--on-node-conflict merge_props` | Fill missing destination properties from source |
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

Useful options:

| Option | Meaning |
|---|---|
| `--key-file PATH` | File containing the HMAC key bytes |
| `-o, --output PATH` | Write the signature JSON to a file instead of stdout |
| `--signature PATH` | Signature JSON file to verify |
| `--format json` | Emit a machine-readable verify report |
| `--force` | Allow overwriting the output signature path |

## Conventions

For files that will be shared or merged repeatedly, use the
[canonicalization conventions](../conventions/canonicalization.md) and
[recommended labels](../conventions/recommended-labels.md). They are not
required, but stable labels and explicit identity properties make command-line
diff and merge results easier to review.
