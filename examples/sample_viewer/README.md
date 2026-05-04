# Sample viewer (read-only)

This folder is a **sample/reference viewer** for `.liel` data. It is intentionally
JSON-first and read-only:

- input: documented JSON from `liel export` only
- rendering: browser UI using embedded JS libraries
- non-goal: parsing `.liel` binary bytes in the browser

The viewer starts with a bundled sample from the
`trace-why-postgres` scenario so you can inspect the UI immediately.

## Quick try

From the repository root:

```bash
python examples/demo_memory/make_demo_files.py --force
liel export target/demo-memory/base.liel -o target/demo-memory/base.export.json
```

Then open `examples/sample_viewer/index.html` in your browser. You can keep the
bundled sample, or load:

- `target/demo-memory/base.export.json`

## GitHub Pages integration

Possible, if wanted later:

1. copy this sample into the published docs/static area (or host as a static asset),
2. keep the same JSON contract boundary (`export` for this sample viewer),
3. avoid browser-side `.liel` parsing.

This repository currently uses MkDocs for local preview and keeps deployment
policy separate; integrate only when that policy is explicitly enabled.
