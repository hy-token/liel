# Inspect your memory

Use this page when you want to answer: **what does this `.liel` file remember,
what changed, and why?** The inspection path stays read-only: commands and the
sample viewer show data, but do not edit the memory file.

## 1. Start with a small summary

```bash
liel stats memory.liel --format json
```

Use this first in CI and local review. It confirms that the file opens and
returns counts, label histograms, file size, and format version.

## 2. Export a reviewable graph snapshot

```bash
liel export memory.liel -o memory.export.json
```

The export JSON is the primary input for read-only viewers and custom reports.
For the built-in sample viewer, use the fixed contract fixture as a known-good
reference:

```text
docs/guide/sample-viewer/app/fixtures/trace-why-postgres.export.json
```

The distributed sample viewer carries the same fixture under:

```text
examples/sample_viewer/fixtures/trace-why-postgres.export.json
```

## 3. Open the read-only sample viewer

Open the viewer from the docs site or from a local checkout:

```text
docs/guide/sample-viewer/app/index.html
```

The viewer loads `trace-why-postgres.export.json` as its bundled sample when it
is served over HTTP. If your browser blocks local `file://` fixture loading, the
viewer falls back to an embedded copy of the same sample; you can also load the
fixture manually with the file picker.

## 4. Compare two memories

```bash
liel diff base.liel incoming.liel --format json --node-key path
```

Use `diff` when you want to review changes without producing a merged file.
Choose `--node-key` or `--identity-rules` to match your project identity policy.

## 5. Preview a merge before writing

```bash
liel merge base.liel incoming.liel \
  --dry-run --fail-on-conflict --format json --node-key path
```

Use this in PR checks when memory changes should not merge automatically if the
preview reports conflicts.

## 6. Trace an impact path

```bash
liel trace memory.liel --from 1 --to 7 --format json
```

`trace` helps explain how a file, task, bug, requirement, decision, or other
node is connected. Text output is useful for humans; JSON output is better for
reports, MCP tools, and CI artifacts.

## Related contract docs

- [Command line](cli.md) — command flags and examples.
- [CLI JSON inventory](../reference/cli-json-inventory.md) — JSON fields and exit codes.
- [Viewer JSON contract](../reference/viewer-json.md) — JSON surfaces for viewers.
- [Sample viewer](sample-viewer.md) — local read-only browser viewer.
