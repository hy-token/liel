"""
liel Knowledge Graph — Heterogeneous Nodes/Edges and QueryBuilder

Node labels : Person(name, role)  /  Company(name, industry)  /  Technology(name, category)
Edge labels : WORKS_AT(since)  /  USES(proficiency)  /  KNOWS
Multiple labels  : a single node can have multiple labels like ["Person", "Manager"]
"""
import liel


def build(db: liel.GraphDB) -> dict:
    # ── Nodes ────────────────────────────────────────────────────────────
    alice = db.add_node(["Person"],            name="Alice", role="Engineer")
    bob   = db.add_node(["Person"],            name="Bob",   role="Designer")
    carol = db.add_node(["Person"],            name="Carol", role="Engineer")
    dave  = db.add_node(["Person", "Manager"], name="Dave",  role="Manager")  # multiple labels

    acme   = db.add_node(["Company"], name="Acme",   industry="SaaS")
    widget = db.add_node(["Company"], name="Widget", industry="FinTech")

    python = db.add_node(["Technology"], name="Python", category="Language")
    rust   = db.add_node(["Technology"], name="Rust",   category="Language")
    react  = db.add_node(["Technology"], name="React",  category="Framework")

    # ── Edges ────────────────────────────────────────────────────────────
    db.add_edge(alice, "WORKS_AT", acme,   since=2021)
    db.add_edge(bob,   "WORKS_AT", acme,   since=2022)
    db.add_edge(carol, "WORKS_AT", widget, since=2020)
    db.add_edge(dave,  "WORKS_AT", widget, since=2019)

    db.add_edge(alice, "USES", python, proficiency="expert")
    db.add_edge(alice, "USES", rust,   proficiency="intermediate")
    db.add_edge(carol, "USES", python, proficiency="intermediate")
    db.add_edge(carol, "USES", rust,   proficiency="expert")
    db.add_edge(bob,   "USES", react,  proficiency="expert")

    db.add_edge(alice, "KNOWS", carol)
    db.add_edge(dave,  "KNOWS", alice)

    db.commit()
    return dict(alice=alice, bob=bob, carol=carol, dave=dave,
                acme=acme, widget=widget, python=python, rust=rust, react=react)


if __name__ == "__main__":
    db = liel.open(":memory:")
    ns = build(db)

    # ── QueryBuilder: Person with Engineer role ──────────────────────────
    engineers = (
        db.nodes()
          .label("Person")
          .where_(lambda n: n.get("role") == "Engineer")
          .fetch()
    )
    print(f"Engineer: {[n['name'] for n in engineers]}")
    # → ['Alice', 'Carol']

    # ── QueryBuilder: Multiple labels (nodes with "Manager" label) ────────
    managers = db.nodes().label("Manager").fetch()
    print(f"Nodes with Manager label: {[n['name'] for n in managers]}")
    # → ['Dave']

    # ── QueryBuilder: Fetch SaaS companies with limit/skip ───────────────
    saas = (
        db.nodes()
          .label("Company")
          .where_(lambda n: n.get("industry") == "SaaS")
          .limit(10)
          .fetch()
    )
    print(f"SaaS companies: {[n['name'] for n in saas]}")
    # → ['Acme']

    # ── BFS: Reachable nodes from Alice (max_depth=2) ─────────────────────
    print("\nBFS from Alice (max_depth=2):")
    for node, depth in db.bfs(ns["alice"], max_depth=2):
        label_str = "/".join(node.labels)
        print(f"{'  ' * depth}{node.get('name')}  [{label_str}]")

    # ── Shortest path: Bob → Carol ────────────────────────────────────────
    path = db.shortest_path(ns["bob"], ns["carol"])
    if path:
        print(f"\nBob → Carol: {' → '.join(n['name'] for n in path)}")
    else:
        print("\nBob → Carol: No path found")
