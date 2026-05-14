# Operations guide: backup, verify, and repair

Use this guide when a `.liel` file is project memory or release evidence and
you need an operator-friendly runbook. It connects the reliability contract,
CLI inspection commands, signing commands, JSON export fallback, and adjacency
repair into one flow.

`liel` is a single-file, single-writer store. The safest operational rule is:
**coordinate the writer first, then copy or verify the closed file**. For the
underlying durability model, see [Reliability and failure model](../reference/reliability.md).

## Fast runbook

| Situation | First action | Command or check |
|-----------|--------------|------------------|
| Routine backup | Stop or coordinate the writer, then copy the closed `.liel` file. | `cp project.liel backups/project-$(date +%Y%m%d-%H%M%S).liel` |
| Pre-release fingerprint | Emit a deterministic manifest from the closed file. | `liel manifest project.liel -o release/project.manifest.json` |
| Signed release evidence | Sign the manifest payload with an external HMAC key. | `liel sign project.liel --key-file release/liel-signing.key -o release/project.liel.sig` |
| Verify an artifact | Verify the file against its signature and key. | `liel verify project.liel --signature release/project.liel.sig --key-file release/liel-signing.key --format json` |
| Human-readable fallback | Export graph records to JSON for review or last-resort migration. | `liel export project.liel -o backups/project.export.json` |
| Suspect adjacency index drift | Take a backup, then run the Python repair API on a copy or maintenance window. | `db.repair_adjacency()` |
| CI smoke | Open every tracked memory and print stable counts. | `liel stats memory.liel --format json` |
| Merge review | Preview a key-aware merge before writing. | `liel merge base.liel incoming.liel --dry-run --fail-on-conflict --format json --node-key path` |

## 1. Prepare a safe maintenance window

Before copying, signing, exporting, or repairing a production memory file:

1. Make sure the owner process has finished writing and has closed the file.
   A closed `with liel.open(...)` block or a stopped MCP server is the clearest
   boundary.
2. Do not copy while another process is actively writing. `liel` protects the
   file with a single-writer guard, but an external copy tool can still observe
   a moment-in-time file image if you run it concurrently.
3. Prefer a local disk for the primary file. Network, cloud-sync, and virtual
   filesystems are outside the recommended durability envelope.
4. Keep the `.liel` file, manifest JSON, signature JSON, and export JSON as
   separate artifacts. A manifest or signature is evidence; it is not a backup.

## 2. Take a closed-file backup

After the writer has closed, copy the file with your platform's normal file copy
command.

=== "macOS / Linux"

    ```bash
    mkdir -p backups
    cp project.liel "backups/project-$(date +%Y%m%d-%H%M%S).liel"
    ```

=== "Windows PowerShell"

    ```powershell
    New-Item -ItemType Directory -Force backups | Out-Null
    Copy-Item project.liel ("backups/project-{0}.liel" -f (Get-Date -Format yyyyMMdd-HHmmss))
    ```

Immediately smoke the copied file:

```bash
liel stats backups/project-20260508-120000.liel --format json
```

If `stats` cannot open the backup, discard that copy and create a new backup
from a known-closed writer boundary.

## 3. Create manifest and signature evidence

A manifest is deterministic JSON for review and signing. It is useful in release
logs because it records the logical graph content, not just a filesystem hash.

```bash
mkdir -p release
liel manifest project.liel -o release/project.manifest.json
```

To sign the graph state, keep an HMAC key outside the repository and produce an
external signature file:

```bash
liel sign project.liel \
  --key-file /secure/path/liel-signing.key \
  -o release/project.liel.sig
```

Verify the artifact before publishing or restoring:

```bash
liel verify project.liel \
  --signature release/project.liel.sig \
  --key-file /secure/path/liel-signing.key \
  --format json
```

Store signing keys in a secret manager or CI secret. Do not commit key material.

## 4. Export JSON as an inspection and migration fallback

`liel export` is not the same as `liel manifest`. Use it when you want the graph
records in a stable JSON shape for inspection, fixture generation, or last-resort
migration:

```bash
liel export project.liel -o backups/project.export.json
```

To confirm the export can rebuild a graph, import it into a new file and smoke
the result:

```bash
liel import backups/project.export.json -o backups/project.roundtrip.liel --format json
liel stats backups/project.roundtrip.liel --format json
```

Keep the exported JSON next to the binary backup when operational risk is high
or when upgrading across a release that changes file-format behavior.

## 5. Repair adjacency indexes only after backing up

`repair_adjacency()` rebuilds adjacency lists from the live edge set. It is a
maintenance API for a narrow class of index inconsistencies; it is not a general
corruption fixer and it does not invent missing nodes or edges.

Recommended repair flow:

1. Stop or coordinate the writer.
2. Copy the original `.liel` file to a backup path.
3. Run the repair on a copy first when possible.
4. Smoke the repaired file with `stats`, `manifest`, and the application query
   that originally failed.
5. Replace the production file only after the repaired copy passes review.

Minimal Python maintenance snippet:

```python
import shutil
from pathlib import Path

import liel

source = Path("project.liel")
repair_copy = Path("maintenance/project.repair.liel")
repair_copy.parent.mkdir(parents=True, exist_ok=True)
shutil.copy2(source, repair_copy)

with liel.open(str(repair_copy)) as db:
    report = db.repair_adjacency()
    db.commit()

print(report)
```

Then verify the repaired copy:

```bash
liel stats maintenance/project.repair.liel --format json
liel manifest maintenance/project.repair.liel -o maintenance/project.repair.manifest.json
```

If opening the file fails with a structural corruption error before repair can
run, restore from the last known-good `.liel` backup or from an exported JSON
round trip. Do not repeatedly run repair against the only copy of a file.

## 6. Failure modes and responses

| Failure mode | What it means | Operator response |
|--------------|---------------|-------------------|
| Process exits before `commit()` | Changes after the last commit are outside the durability contract. | Reopen the file, confirm state with `liel stats`, and replay the application-level operation if needed. |
| Writer crashes after WAL fsync | Committed changes may need WAL replay. | Reopen once with `liel stats`; recovery runs during open. Then create a fresh backup. |
| Same file opened by another writer | Single-writer guard rejected the second writer. | Stop duplicate writers or route writes through one MCP server / owner process. |
| Stale `.lock/` after crash | Previous owner died before removing the guard directory. | Reopen normally; `liel` reclaims clearly dead owners. If the owner may still be alive, investigate before deleting anything manually. |
| `liel verify` fails | File content no longer matches the signature payload and key. | Treat as changed or wrong artifact. Recompute manifest for investigation; do not publish as the signed artifact. |
| Header checksum, invalid magic, unsupported version, or truncation error | File cannot be interpreted safely. | Stop writing, preserve the bad file for diagnosis, restore from the latest known-good backup or JSON export. |
| Suspected adjacency drift but graph opens | Edge records exist, but traversal indexes may be inconsistent. | Backup first, run `repair_adjacency()` on a copy, then smoke and review. |
| WAL reservation exceeded | A single transaction is too large for the fixed WAL reservation. | Split imports or bulk updates into smaller committed batches. |
| Network/cloud-sync copy looks inconsistent | External storage observed or reordered file operations outside the comfort zone. | Move the primary file to local disk, restore from a known-good backup, and copy only closed files. |

## 7. CI and release smoke

For CI, prefer lightweight checks that prove the file opens and emits stable JSON:

```bash
liel stats project.liel --format json
liel manifest project.liel -o artifacts/project.manifest.json
```

For release evidence, the accepted `1.0` smoke set also records signature
verification plus export/import round-trip viability. Add merge preview only when
your repository has a merge policy or identity key to enforce:

```bash
liel verify project.liel \
  --signature release/project.liel.sig \
  --key-file /secure/path/liel-signing.key \
  --format json

liel export project.liel > artifacts/project.export.json
liel import artifacts/project.export.json -o artifacts/project.imported.liel \
  --format json

# Required only when this project publishes a merge identity policy.
liel merge base.liel incoming.liel \
  --dry-run --fail-on-conflict --format json --node-key path
```

Copyable GitHub Actions examples live in
[`examples/github-actions/`](https://github.com/hy-token/liel/tree/main/examples/github-actions),
and the CI guide explains how to wire them into a repository.
