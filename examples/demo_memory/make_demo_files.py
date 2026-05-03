"""
Generate fixed `.liel` files for Phase 4 demo assets.

Run from a checkout after `maturin develop` or another local install:

    python examples/demo_memory/make_demo_files.py --force
    python examples/demo_memory/make_demo_files.py --force --extras

`--extras` adds `merge-fail-left.liel` / `merge-fail-right.liel` for
`liel merge --dry-run --fail-on-conflict` tapes (`demos/demo-ci-merge-fail.*`).

Generated `.liel` files live under `target/demo-memory/` by default and are
ignored by git. The tracked source of truth is this script plus the VHS tapes in
`demos/`.

Also writes `trace-why-postgres.liel`: a seven-node billing / PostgreSQL
decision graph for `demos/demo-trace.*.tape` (`liel trace --from 1 --to 7`).
"""

from __future__ import annotations

import argparse
import json
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import liel


@dataclass(frozen=True)
class NodeSpec:
    ref: str
    labels: list[str]
    properties: dict[str, Any]


@dataclass(frozen=True)
class EdgeSpec:
    from_ref: str
    label: str
    to_ref: str
    properties: dict[str, Any]


IDENTITY_RULES = {
    "identity_rules": {
        "Bug": ["key"],
        "Decision": ["key"],
        "Dependency": ["key"],
        "File": ["key"],
        "Service": ["key"],
        "Task": ["key"],
    }
}


BASE_NODES = [
    NodeSpec("auth", ["Service"], {"key": "service:auth", "name": "Auth API"}),
    NodeSpec("billing", ["Service"], {"key": "service:billing", "name": "Billing API"}),
    NodeSpec("postgres", ["Dependency"], {"key": "dep:postgres", "name": "PostgreSQL"}),
    NodeSpec("redis", ["Dependency"], {"key": "dep:redis", "name": "Redis"}),
    NodeSpec("stripe", ["Dependency"], {"key": "dep:stripe", "name": "Stripe"}),
    NodeSpec(
        "decision-db",
        ["Decision"],
        {
            "key": "decision:session-store",
            "title": "Use PostgreSQL for account and session state",
            "reason": "Keep transactional auth and billing updates in one durable store",
        },
    ),
    NodeSpec(
        "bug-stripe",
        ["Bug"],
        {
            "key": "bug:stripe-duplicate-webhook",
            "title": "Stripe webhook duplicate billing",
            "status": "open",
            "owner": "billing",
        },
    ),
    NodeSpec(
        "file-webhook",
        ["File"],
        {
            "key": "file:services/billing/webhooks.py",
            "path": "services/billing/webhooks.py",
            "role": "Stripe webhook handler",
        },
    ),
    NodeSpec(
        "task-observe",
        ["Task"],
        {
            "key": "task:add-webhook-idempotency-log",
            "title": "Add idempotency logging for Stripe webhooks",
            "status": "todo",
        },
    ),
]

BASE_EDGES = [
    EdgeSpec("auth", "USES", "postgres", {}),
    EdgeSpec("billing", "USES", "postgres", {}),
    EdgeSpec("billing", "CALLS", "stripe", {}),
    EdgeSpec("bug-stripe", "AFFECTS", "billing", {}),
    EdgeSpec("bug-stripe", "TOUCHES", "file-webhook", {}),
    EdgeSpec("task-observe", "MITIGATES", "bug-stripe", {}),
    EdgeSpec("decision-db", "SUPPORTS", "auth", {}),
]

# Seven-node “Why PostgreSQL for billing?” graph for `liel trace` GIFs only.
# Node creation order fixes stable IDs 1–7 for `demos/demo-trace.*.tape`.
TRACE_WHY_POSTGRES_NODES = [
    NodeSpec(
        "task-billing",
        ["Task"],
        {
            "key": "task:billing-service",
            "title": "Implement billing service",
            "status": "done",
            "trace_prompt": "Why PostgreSQL for billing?",
        },
    ),
    NodeSpec(
        "option-pg",
        ["Option"],
        {
            "key": "option:postgres",
            "title": "Use PostgreSQL",
            "summary": "ACID transactions and strong consistency",
            "key_factor": "ACID transactions",
        },
    ),
    NodeSpec(
        "option-ddb",
        ["Option"],
        {
            "key": "option:dynamodb",
            "title": "Use DynamoDB",
            "summary": "Scalable and operationally simple",
            "rejection_note": "better for scale, not for this use case",
        },
    ),
    NodeSpec(
        "bug-dup",
        ["Bug"],
        {
            "key": "bug:duplicate-charge",
            "title": "Duplicate charge incident",
            "severity": "high",
            "summary": "Previous eventual consistency issue caused double billing",
        },
    ),
    NodeSpec(
        "req-audit",
        ["Requirement"],
        {
            "key": "requirement:audit-trail",
            "title": "Audit trail requirement",
            "source": "SOC2 review",
            "summary": "Billing records must be traceable and immutable",
        },
    ),
    NodeSpec(
        "decision-pg",
        ["Decision"],
        {
            "key": "decision:billing-postgres",
            "title": "Choose PostgreSQL for billing",
            "reason": (
                "ACID transactions and strong consistency; "
                "Prevents duplicate charges after the prior incident; "
                "Supports auditability and immutable billing records"
            ),
        },
    ),
    NodeSpec(
        "file-billing-db",
        ["File"],
        {
            "key": "file:billing/db.py",
            "path": "billing/db.py",
            "summary": "Billing persistence layer",
        },
    ),
]

# Edges from `task-billing` are ordered so **CONSIDERED → PostgreSQL** is added
# last: `out_edges` uses reverse insertion order, so BFS shortest path prefers
# task → option-pg → decision → file over the equal-length path via the bug.
TRACE_WHY_POSTGRES_EDGES = [
    EdgeSpec("option-pg", "SUPPORTS", "decision-pg", {}),
    EdgeSpec("option-ddb", "REJECTED_FOR", "decision-pg", {}),
    EdgeSpec("bug-dup", "MOTIVATED", "decision-pg", {}),
    EdgeSpec("req-audit", "REQUIRED", "decision-pg", {}),
    EdgeSpec("decision-pg", "IMPLEMENTED_IN", "file-billing-db", {}),
    EdgeSpec("task-billing", "LEARNED_FROM", "bug-dup", {}),
    EdgeSpec("task-billing", "CONSTRAINED_BY", "req-audit", {}),
    EdgeSpec("task-billing", "CONSIDERED", "option-ddb", {}),
    EdgeSpec("task-billing", "CONSIDERED", "option-pg", {}),
]

AGENT_A_NODES = [
    NodeSpec(
        "bug-stripe",
        ["Bug"],
        {
            "key": "bug:stripe-duplicate-webhook",
            "title": "Stripe webhook duplicate billing",
            "status": "investigating",
            "owner": "billing",
        },
    ),
    NodeSpec(
        "task-auth",
        ["Task"],
        {
            "key": "task:move-auth-sessions-to-postgres",
            "title": "Move auth sessions to PostgreSQL",
            "status": "in_progress",
        },
    ),
    NodeSpec(
        "file-auth",
        ["File"],
        {
            "key": "file:services/auth/session_store.py",
            "path": "services/auth/session_store.py",
            "role": "Session persistence",
        },
    ),
]

AGENT_A_EDGES = [
    EdgeSpec("task-auth", "IMPLEMENTS", "decision-db", {}),
    EdgeSpec("task-auth", "TOUCHES", "file-auth", {}),
    EdgeSpec("file-auth", "DEPENDS_ON", "postgres", {}),
]

AGENT_B_NODES = [
    NodeSpec(
        "bug-stripe",
        ["Bug"],
        {
            "key": "bug:stripe-duplicate-webhook",
            "title": "Stripe webhook duplicate billing",
            "status": "fix_ready",
            "owner": "billing",
        },
    ),
    NodeSpec(
        "task-billing",
        ["Task"],
        {
            "key": "task:make-stripe-webhook-idempotent",
            "title": "Make Stripe webhook processing idempotent",
            "status": "review",
        },
    ),
    NodeSpec(
        "decision-retry",
        ["Decision"],
        {
            "key": "decision:stripe-retry-policy",
            "title": "Treat Stripe webhook retries as expected traffic",
            "reason": "External delivery is at-least-once, so handlers must dedupe",
        },
    ),
]

AGENT_B_EDGES = [
    EdgeSpec("task-billing", "MITIGATES", "bug-stripe", {}),
    EdgeSpec("task-billing", "TOUCHES", "file-webhook", {}),
    EdgeSpec("decision-retry", "SUPPORTS", "task-billing", {}),
    EdgeSpec("billing", "USES", "redis", {"purpose": "short-lived webhook dedupe cache"}),
]


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate fixed .liel files for demos.")
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("target/demo-memory"),
        help="Output directory for generated demo files.",
    )
    parser.add_argument("--force", action="store_true", help="Overwrite an existing output dir.")
    parser.add_argument(
        "--clean-locks",
        action="store_true",
        help="Remove generated .liel.lock directories from the output directory and exit.",
    )
    parser.add_argument(
        "--extras",
        action="store_true",
        help=(
            "Also write merge-fail-left.liel / merge-fail-right.liel for "
            "`liel merge --dry-run --fail-on-conflict` demos (minimal CI conflict pair)."
        ),
    )
    args = parser.parse_args()

    if args.clean_locks:
        _clean_locks(args.out)
        print(f"Removed generated lock directories under {args.out}")
        return

    if args.out.exists():
        if not args.force:
            raise SystemExit(f"{args.out} already exists; pass --force to overwrite it")
        shutil.rmtree(args.out)
    args.out.mkdir(parents=True)

    _write_graph(args.out / "base.liel", BASE_NODES, BASE_EDGES)
    _write_graph(
        args.out / "trace-why-postgres.liel",
        TRACE_WHY_POSTGRES_NODES,
        TRACE_WHY_POSTGRES_EDGES,
    )
    _write_graph(
        args.out / "agent-a.liel",
        _overlay_nodes(BASE_NODES, AGENT_A_NODES),
        BASE_EDGES + AGENT_A_EDGES,
    )
    _write_graph(
        args.out / "agent-b.liel",
        _overlay_nodes(BASE_NODES, AGENT_B_NODES),
        BASE_EDGES + AGENT_B_EDGES,
    )
    (args.out / "identity-rules.json").write_text(
        json.dumps(IDENTITY_RULES, indent=2) + "\n",
        encoding="utf-8",
    )

    if args.extras:
        _write_merge_fail_ci_pair(args.out)

    print(f"Generated demo memory files in {args.out}")
    print()
    print("Try:")
    print(f"  liel stats {args.out / 'base.liel'}")
    print(
        "  liel diff "
        f"{args.out / 'base.liel'} {args.out / 'agent-a.liel'} "
        f"--identity-rules {args.out / 'identity-rules.json'}"
    )
    print(
        "  liel merge "
        f"{args.out / 'agent-a.liel'} {args.out / 'agent-b.liel'} "
        f"--dry-run --identity-rules {args.out / 'identity-rules.json'} "
        "--edge-strategy idempotent"
    )
    twp = args.out / "trace-why-postgres.liel"
    print(f"  liel trace {twp} --from 1 --to 7 --no-mermaid")
    print(f"  liel trace {twp} --from 1 --to 7")
    print(
        "    (Why PostgreSQL for billing: task -> option PG -> decision -> file; "
        "sync with demos/demo-trace.*.tape)"
    )
    if args.extras:
        print(
            "  liel merge "
            f"{args.out / 'merge-fail-left.liel'} {args.out / 'merge-fail-right.liel'} "
            "--node-key tag --dry-run --fail-on-conflict"
        )


def _write_merge_fail_ci_pair(out: Path) -> None:
    """Minimal pair where merge preview is blocked (matches CLI tests)."""
    left = out / "merge-fail-left.liel"
    right = out / "merge-fail-right.liel"
    _remove_generated_file(left)
    _remove_generated_file(right)
    with liel.open(str(left)) as db:
        db.add_node(["Item"], tag="A")
        db.commit()
    with liel.open(str(right)) as db:
        db.add_node(["Item"], name="missing")
        db.commit()
    _remove_generated_lock(left)
    _remove_generated_lock(right)


def _write_graph(path: Path, nodes: list[NodeSpec], edges: list[EdgeSpec]) -> None:
    _remove_generated_file(path)
    with liel.open(str(path)) as db:
        created: dict[str, liel.Node] = {}
        for spec in nodes:
            created[spec.ref] = db.add_node(spec.labels, **spec.properties)
        for spec in edges:
            db.merge_edge(
                created[spec.from_ref],
                spec.label,
                created[spec.to_ref],
                **spec.properties,
            )
        db.commit()
    _remove_generated_lock(path)


def _overlay_nodes(base: list[NodeSpec], overrides: list[NodeSpec]) -> list[NodeSpec]:
    by_ref = {spec.ref: spec for spec in base}
    for spec in overrides:
        by_ref[spec.ref] = spec
    return list(by_ref.values())


def _remove_generated_file(path: Path) -> None:
    path.unlink(missing_ok=True)
    _remove_generated_lock(path)


def _clean_locks(out: Path) -> None:
    if not out.exists():
        return
    for lock_dir in out.glob("*.liel.lock"):
        shutil.rmtree(lock_dir, ignore_errors=True)


def _remove_generated_lock(path: Path) -> None:
    shutil.rmtree(Path(str(path) + ".lock"), ignore_errors=True)


if __name__ == "__main__":
    main()
