# Sample viewer (read-only)

This page documents the **sample/reference viewer** for `.liel` data.
The interactive HTML lives in `docs/guide/sample-viewer/app/` so it does not
collide with this MkDocs page at `/guide/sample-viewer/`.

**[Open the interactive viewer](sample-viewer/app/index.html)** (export JSON only).

Design stance:

- input: documented JSON from `liel export` only
- rendering: browser UI using embedded JS libraries
- non-goal: parsing `.liel` binary bytes in the browser

The viewer starts with the fixed `trace-why-postgres.export.json` fixture from the
`trace-why-postgres` scenario so you can inspect the UI immediately. The docs
viewer reads it from `docs/guide/sample-viewer/app/fixtures/`; the distributed
example carries the same file under `examples/sample_viewer/fixtures/`.

## Quick try

From the repository root:

```bash
python examples/demo_memory/make_demo_files.py --force
liel export target/demo-memory/base.liel -o target/demo-memory/base.export.json
```

Then open `docs/guide/sample-viewer/app/index.html` in your browser. You can keep the
bundled fixture, or load:

- `docs/guide/sample-viewer/app/fixtures/trace-why-postgres.export.json`
- `target/demo-memory/base.export.json`

## MkDocs / local preview / GitHub Pages

This viewer is under `docs/`. With `site_url` set to the GitHub Pages project
site, `mkdocs serve` uses the `/liel/` prefix (same as production):

- documentation (this page): `http://127.0.0.1:8000/liel/guide/sample-viewer/`
- interactive viewer: `http://127.0.0.1:8000/liel/guide/sample-viewer/app/`

Published site (after deploy from `hy-token/liel`): `https://hy-token.github.io/liel/guide/sample-viewer/`
and `https://hy-token.github.io/liel/guide/sample-viewer/app/`.

## Contract fixture

The viewer, docs, and contract tests share the fixed
`trace-why-postgres.export.json` export fixture. If `liel export` changes shape
intentionally, update the fixture, [Viewer JSON contract](../reference/viewer-json.md),
and `tests/python/test_viewer_fixture_contract.py` together.
