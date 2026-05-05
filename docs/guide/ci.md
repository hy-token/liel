# CI and GitHub Actions

Use the `liel` CLI in continuous integration to treat `.liel` memory files like
other reviewable artifacts: confirm they open cleanly, compare versions, or
block merges when a preview reports conflicts.

## When to use which command

| Goal | Command | Notes |
|------|---------|------|
| Smoke-test that files open; counts and label distribution | `liel stats --format json` | Lightest check; used in the sample workflow below |
| Compare two checked-in memories | `liel diff --format json` | Pair arbitrary paths (e.g. base branch vs PR branch checkout) |
| Check merge safety before integrating branches | `liel merge --dry-run` (optional `--fail-on-conflict`) | Align `--node-key` / `--identity-rules` with your identity policy |
| Deterministic fingerprint for signing | `liel manifest` (often paired with `liel sign` / `liel verify`) | Same manifest JSON as MCP `liel_manifest` |

Field-level references: [CLI JSON inventory](../reference/cli-json-inventory.md),
[CLI merge report](../reference/cli-merge-report.md).

## Install `liel` in a workflow

On GitHub-hosted runners, install the published wheel after setting up Python:

```yaml
- uses: actions/setup-python@v6
  with:
    python-version: "3.11"
- run: pip install "liel>=0.6,<1"
```

Then invoke `liel` the same way you would locally. Exit codes follow
[CLI JSON inventory](../reference/cli-json-inventory.md) and command-specific
references (for example [CLI merge report](../reference/cli-merge-report.md)
for merge previews).

## Sample workflow: stats for every tracked `.liel` file

A minimal check runs `liel stats --format json` on each tracked graph file.
That verifies the file opens and emits stable summary JSON (counts, label
histograms, format version, file size).

Example line printed to the job log (one JSON object per line; field definitions
in [CLI JSON inventory](../reference/cli-json-inventory.md) under **liel stats**):

```json
{"edge_labels": {}, "edge_count": 0, "file_size": 8192, "liel_format": "1.0", "node_count": 2, "node_labels": {"Task": 2}, "path": "/home/runner/work/repo/memory.liel"}
```

Copy the template from the repository:

- **Source file:** [`examples/github-actions/liel-memory-check.yml`](https://github.com/hy-token/liel/blob/main/examples/github-actions/liel-memory-check.yml)
  (adjust the URL if you fork or mirror).

Place it under `.github/workflows/` in your project—for example
`.github/workflows/liel-memory-check.yml`.

If the repository has **no** tracked `*.liel` files, the sample workflow exits
successfully after printing a short message.

## Sample workflow: manifest JSON for every tracked `.liel` file

For a deterministic fingerprint per file (useful for release logs or comparing
checkouts), run `liel manifest --format json` on each tracked graph file. Field
definitions are in [CLI JSON inventory](../reference/cli-json-inventory.md)
under **liel manifest**.

Copy the template:

- **Source file:** [`examples/github-actions/liel-memory-manifest.yml`](https://github.com/hy-token/liel/blob/main/examples/github-actions/liel-memory-manifest.yml)

You can use **both** this workflow and the stats workflow above if you want
open checks plus manifest lines in CI.

## Merge preview and failing on conflicts

To enforce that a key-aware merge preview is **not blocked** before merging a
branch, run `liel merge` with `--dry-run` and [`--fail-on-conflict`](cli.md#merge)
against your chosen base and topic files. Typical pattern:

```bash
liel merge base.liel incoming.liel --dry-run --fail-on-conflict --format json \
  --node-key path
```

Tune `--node-key`, `--identity-rules`, and other merge flags to match how your
team identifies nodes across branches. The JSON payload is unchanged; only the
process exit code becomes non-zero when `can_merge` is `false` or `conflicts`
is non-empty.

For signing workflows (`liel sign` / `liel verify`), store keys using GitHub
**secrets** and pass them to non-interactive verify steps; never commit secret
material.

## Related documentation

- [Command line](cli.md) — full CLI reference
- [CLI JSON inventory](../reference/cli-json-inventory.md) — exit codes and JSON roles
- [Reliability](../reference/reliability.md) — crash recovery and commit semantics
