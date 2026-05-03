# GitHub Actions samples for `.liel` files

These YAML files are **copy-paste templates** for projects that keep agent or
project memory in `.liel` files. They are not run as workflows inside the
`liel-dev` repository by default.

| File | Purpose |
|------|---------|
| [liel-memory-check.yml](liel-memory-check.yml) | On each push/PR, run `liel stats --format json` on every tracked `*.liel` file. |
| [liel-memory-manifest.yml](liel-memory-manifest.yml) | On each push/PR, run `liel manifest --format json` on every tracked `*.liel` file (deterministic fingerprint for release or audit logs). |

**Setup:** copy the file into your repo under `.github/workflows/` and commit.

See the user guide: [CI / GitHub Actions](../../docs/guide/ci.md).
