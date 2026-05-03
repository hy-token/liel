# Demo Memory Data

This directory contains fixed SaaS-shaped demo data for Phase 4 distribution
assets. It is designed for README and GitHub Pages demos that need to show real
implemented `liel` commands without hand-recorded state.

Generate the `.liel` files from a checkout after installing the package locally:

```bash
python examples/demo_memory/make_demo_files.py --force
```

The generated files are written to `target/demo-memory/` by default and are
ignored by git:

| File | Purpose |
|---|---|
| `base.liel` | Shared project memory before two agents branch |
| `agent-a.liel` | Agent A adds auth session work and updates the Stripe bug |
| `agent-b.liel` | Agent B adds billing retry work and updates the same Stripe bug |
| `identity-rules.json` | Stable identities for key-aware diff and merge |

Useful implemented commands:

```bash
liel stats target/demo-memory/base.liel
liel diff target/demo-memory/base.liel target/demo-memory/agent-a.liel --identity-rules target/demo-memory/identity-rules.json
liel merge target/demo-memory/agent-a.liel target/demo-memory/agent-b.liel --dry-run --identity-rules target/demo-memory/identity-rules.json --edge-strategy idempotent
```

The data uses a small SaaS product memory:

- services: auth, billing, postgres, redis, stripe
- memory: bugs, decisions, files, tasks, dependencies
- scenario: two agents safely preview a memory merge before applying it
