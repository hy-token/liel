"""
liel × pandas — DataFrame conversion and aggregation

Requires: pip install pandas
"""

import pandas as pd

import liel

# ─── Conversion Utilities ───────────────────────────────────────────────

def nodes_to_df(db: liel.GraphDB) -> pd.DataFrame:
    """Convert all nodes to a DataFrame. Columns: id, labels, + each property key."""
    return pd.DataFrame(db.all_nodes_as_records())


def edges_to_df(db: liel.GraphDB) -> pd.DataFrame:
    """Convert all edges to a DataFrame. Columns: id, label, from_node, to_node, + each property key."""
    return pd.DataFrame(db.all_edges_as_records())


def degree_df(db: liel.GraphDB) -> pd.DataFrame:
    """Return a DataFrame with out-degree and in-degree for each node."""
    degrees = db.degree_stats()  # {node_id: (out_degree, in_degree)} — aggregated on the Rust side
    rows = db.all_nodes_as_records()
    for r in rows:
        r["out_degree"], r["in_degree"] = degrees.get(r["id"], (0, 0))
    return pd.DataFrame(rows)


# ─── Demo: Social Graph ─────────────────────────────────────────────────

def build_social_graph(db):
    people = [
        ("Alice", 30, "Engineer"),
        ("Bob",   25, "Designer"),
        ("Carol", 35, "Manager"),
        ("Dave",  28, "Engineer"),
        ("Eve",   32, "Designer"),
        ("Frank", 40, "Manager"),
    ]
    nodes = {}
    for name, age, role in people:
        nodes[name] = db.add_node(["Person"], name=name, age=age, role=role)

    follows = [
        ("Alice", "Bob",   2020),
        ("Alice", "Carol", 2019),
        ("Bob",   "Dave",  2021),
        ("Carol", "Dave",  2022),
        ("Carol", "Eve",   2020),
        ("Dave",  "Frank", 2023),
        ("Eve",   "Frank", 2021),
        ("Frank", "Alice", 2018),
    ]
    for frm, to, since in follows:
        db.add_edge(nodes[frm], "FOLLOWS", nodes[to], since=since)

    db.commit()
    return nodes


if __name__ == "__main__":
    db = liel.open(":memory:")
    build_social_graph(db)

    # ── Node list ────────────────────────────────────────────────────────
    node_df = nodes_to_df(db)
    print("=== nodes ===")
    print(node_df.to_string(index=False))
    print()

    # ── Edge list ────────────────────────────────────────────────────────
    edge_df = edges_to_df(db)
    print("=== edges ===")
    print(edge_df.to_string(index=False))
    print()

    # ── Average age by role ──────────────────────────────────────────────
    print("=== Average age by role ===")
    print(node_df.groupby("role")["age"].mean().to_string())
    print()

    # ── Follower count ranking ───────────────────────────────────────────
    deg = degree_df(db)
    print("=== Follower count ranking ===")
    print(
        deg.sort_values("in_degree", ascending=False)
           .to_string(index=False)
    )
    print()

    # ── QueryBuilder: Engineers aged 30 and above ────────────────────────
    seniors = (
        db.nodes()
          .label("Person")
          .where_(lambda n: n.get("age") >= 30 and n.get("role") == "Engineer")
          .fetch()
    )
    print("=== Engineers aged 30 and above ===")
    for n in seniors:
        print(f"  {n['name']}  age={n['age']}")
    print()

    # ── Example: convert since column to datetime ────────────────────────
    edge_df["since_dt"] = pd.to_datetime(edge_df["since"], format="%Y")
    print("=== edges with since_dt ===")
    print(edge_df[["from_node", "to_node", "since", "since_dt"]].to_string(index=False))
