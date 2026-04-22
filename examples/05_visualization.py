"""
liel × networkx × matplotlib — Graph Visualization

Requires: pip install networkx matplotlib
"""

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import networkx as nx

import liel

# ─── networkx conversion ────────────────────────────────────────────────

def to_networkx(db: liel.GraphDB, directed: bool = True) -> nx.Graph:
    """Convert a liel.GraphDB to a networkx graph."""
    G: nx.Graph = nx.DiGraph() if directed else nx.Graph()
    for r in db.all_nodes_as_records():
        G.add_node(r["id"], labels=r["labels"],
                   **{k: v for k, v in r.items() if k not in ("id", "labels")})
    for r in db.all_edges_as_records():
        G.add_edge(r["from_node"], r["to_node"], label=r["label"],
                   **{k: v for k, v in r.items() if k not in ("id", "label", "from_node", "to_node")})
    return G


# ─── Drawing Utilities ──────────────────────────────────────────────────

def draw_graph(
    db: liel.GraphDB,
    *,
    label_prop: str = "name",
    color_by: str | None = None,
    title: str = "",
    figsize: tuple[int, int] = (10, 7),
    show_edge_labels: bool = True,
) -> None:
    """Draw a liel graph using matplotlib."""
    G = to_networkx(db)
    pos = nx.spring_layout(G, seed=42)

    node_labels = {n: G.nodes[n].get(label_prop, str(n)) for n in G.nodes()}

    if color_by:
        categories = sorted({G.nodes[n].get(color_by, "?") for n in G.nodes()})
        palette = plt.cm.Set2
        color_map = {c: palette(i / max(len(categories) - 1, 1)) for i, c in enumerate(categories)}
        node_colors = [color_map[G.nodes[n].get(color_by, "?")] for n in G.nodes()]
    else:
        node_colors = ["#4C72B0"] * len(G.nodes())

    fig, ax = plt.subplots(figsize=figsize)
    nx.draw_networkx_nodes(G, pos, node_color=node_colors, node_size=900, alpha=0.9, ax=ax)
    nx.draw_networkx_labels(G, pos, labels=node_labels, font_size=9, ax=ax)
    nx.draw_networkx_edges(
        G, pos, edge_color="#888888", arrows=True,
        arrowsize=18, ax=ax, connectionstyle="arc3,rad=0.08",
    )

    if show_edge_labels:
        edge_labels = {(u, v): G.edges[u, v].get("label", "") for u, v in G.edges()}
        nx.draw_networkx_edge_labels(G, pos, edge_labels=edge_labels, font_size=7, ax=ax)

    if color_by:
        patches = [mpatches.Patch(color=color_map[c], label=c) for c in categories]
        ax.legend(handles=patches, loc="upper left", fontsize=8)

    ax.set_title(title)
    ax.axis("off")
    plt.tight_layout()
    plt.show()


def draw_bfs_tree(db: liel.GraphDB, start_node, max_depth: int = 3, *, title: str = "") -> None:
    """Draw BFS results with depth-based coloring."""
    start_id = start_node.id if hasattr(start_node, "id") else start_node
    start = db.get_node(start_id) if not hasattr(start_node, "id") else start_node
    result = db.bfs(start_node, max_depth)  # [(Node, depth), ...]
    depth_map = {start_id: 0}
    depth_map.update({n.id: d for n, d in result})

    G = nx.DiGraph()
    G.add_node(start_id, name=start.get("name") or str(start_id), depth=0)
    for node, depth in result:
        G.add_node(node.id, name=node.get("name") or str(node.id), depth=depth)

    for r in db.edges_between(depth_map.keys()):
        G.add_edge(r["from_node"], r["to_node"], label=r["label"])

    pos = nx.bfs_layout(G, start=[start_id])
    node_labels = {n: G.nodes[n]["name"] for n in G.nodes()}
    depths = [depth_map.get(n, 0) for n in G.nodes()]
    colors = plt.cm.Blues([0.3 + 0.2 * d for d in depths])

    fig, ax = plt.subplots(figsize=(10, 6))
    nx.draw_networkx_nodes(G, pos, node_color=colors, node_size=800, ax=ax)
    nx.draw_networkx_labels(G, pos, labels=node_labels, font_size=9, ax=ax)
    nx.draw_networkx_edges(G, pos, arrows=True, ax=ax)
    ax.set_title(title or f"BFS tree (max_depth={max_depth})")
    ax.axis("off")
    plt.tight_layout()
    plt.show()


# ─── Demo 1: Social Graph ────────────────────────────────────────────────

def demo_social():
    db = liel.open(":memory:")
    people = [
        ("Alice", "Engineer"),
        ("Bob",   "Designer"),
        ("Carol", "Manager"),
        ("Dave",  "Engineer"),
        ("Eve",   "Designer"),
        ("Frank", "Manager"),
    ]
    nodes = {}
    for name, role in people:
        nodes[name] = db.add_node(["Person"], name=name, role=role)

    for frm, to in [
        ("Alice", "Bob"),   ("Alice", "Carol"),
        ("Bob",   "Dave"),  ("Carol", "Dave"),
        ("Carol", "Eve"),   ("Eve",   "Frank"),
    ]:
        db.add_edge(nodes[frm], "FOLLOWS", nodes[to])
    db.commit()

    draw_graph(db, label_prop="name", color_by="role",
               title="Social graph — color by role")

    draw_bfs_tree(db, nodes["Alice"], max_depth=3, title="BFS from Alice")


# ─── Demo 2: Dependency Graph ────────────────────────────────────────────

def demo_deps():
    db = liel.open(":memory:")
    pkgs = ["myapp", "requests", "urllib3", "certifi", "charset-normalizer", "idna"]
    ns = {p: db.add_node(["Package"], name=p) for p in pkgs}
    for frm, to in [
        ("myapp",    "requests"),
        ("requests", "urllib3"),
        ("requests", "certifi"),
        ("requests", "charset-normalizer"),
        ("requests", "idna"),
    ]:
        db.add_edge(ns[frm], "DEPENDS_ON", ns[to])
    db.commit()

    draw_graph(db, label_prop="name", title="Package dependency graph", figsize=(9, 5))


# ─── Demo 3: Using networkx analysis API directly ────────────────────────

def demo_nx_analysis():
    db = liel.open(":memory:")
    names = ["A", "B", "C", "D", "E"]
    ns = {n: db.add_node(["Node"], name=n) for n in names}
    for frm, to in [("A","B"),("A","C"),("B","D"),("C","D"),("D","E"),("E","A")]:
        db.add_edge(ns[frm], "LINK", ns[to])
    db.commit()

    G = to_networkx(db)
    print("PageRank:")
    pr = nx.pagerank(G)
    node_names = {n.id: n.get("name") for n in db.all_nodes()}
    for nid, score in sorted(pr.items(), key=lambda x: -x[1]):
        print(f"  {node_names[nid]}: {score:.4f}")

    print("\nCycle detection:", list(nx.simple_cycles(G)))


if __name__ == "__main__":
    demo_social()
    demo_deps()
    demo_nx_analysis()
