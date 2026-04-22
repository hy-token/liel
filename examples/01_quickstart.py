"""
liel Quickstart — CRUD and Basic Operations

What you'll learn:
  - Adding, retrieving, updating, and deleting nodes/edges
  - Transaction control with commit / rollback
  - Adjacency queries (neighbors, out_edges, in_edges)
"""
import liel

with liel.open("example_1.liel") as db:
    db.clear()

    # ── Add ──────────────────────────────────────────────────────────────
    alice = db.add_node(["Person"], name="Alice", age=30)
    bob   = db.add_node(["Person"], name="Bob",   age=25)
    carol = db.add_node(["Person"], name="Carol", age=35)
    e1    = db.add_edge(alice, "KNOWS", bob,   since=2020)
    db.add_edge(bob, "KNOWS", carol, since=2022)
    db.commit()
    print(f"{db.node_count()} nodes, {db.edge_count()} edges")  # 3 nodes, 2 edges

    # ── Get ──────────────────────────────────────────────────────────────
    n = db.get_node(alice.id)
    print(f"{n['name']}  age={n['age']}")            # Alice  age=30

    e = db.get_edge(e1.id)
    print(f"{e.label}  since={e['since']}")          # KNOWS  since=2020

    # ── Adjacency Queries ────────────────────────────────────────────────
    friends = db.neighbors(alice, edge_label="KNOWS")
    print([f["name"] for f in friends])              # ['Bob']

    print(db.out_edges(alice)[0].label)              # KNOWS
    print(len(db.in_edges(bob)))                     # 1

    # ── Update ───────────────────────────────────────────────────────────
    db.update_node(alice.id, name="Alice", age=31)   # birthday
    print(db.get_node(alice.id)["age"])              # 31

    # ── Delete and rollback ──────────────────────────────────────────────
    db.delete_node(carol.id)
    print(db.node_count())                           # 2
    db.rollback()
    print(db.node_count())                           # 3 — carol restored
