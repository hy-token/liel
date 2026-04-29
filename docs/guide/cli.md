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

## Conventions

For files that will be shared or merged repeatedly, use the
[canonicalization conventions](../conventions/canonicalization.md) and
[recommended labels](../conventions/recommended-labels.md). They are not
required, but stable labels and explicit identity properties make command-line
diff and merge results easier to review.
