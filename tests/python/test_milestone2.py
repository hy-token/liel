def test_delete_node(db):
    """delete_node() must remove the node so that get_node() returns None afterward.

    After deleting a node by its ID, any subsequent call to get_node() with
    that same ID must return None.
    """
    alice = db.add_node(["Person"])
    db.delete_node(alice.id)
    assert db.get_node(alice.id) is None


def test_delete_node_removes_edges(db):
    """Deleting a node must also remove all edges incident to that node.

    When the source node of an edge is deleted, the edge must be removed from
    the adjacency list of the target node.  in_edges / out_edges on the
    remaining node must reflect the deletion.
    """
    alice = db.add_node([])
    bob = db.add_node([])
    db.add_edge(alice, "KNOWS", bob)
    db.delete_node(alice.id)
    # The incoming edge to bob must be gone after alice is deleted.
    assert len(db.out_edges(bob)) == 0


def test_delete_edge(db):
    """delete_edge() must remove the edge from both get_edge() and out_edges().

    After deleting an edge, get_edge() must return None for that edge's ID,
    and out_edges() on the source node must no longer include it.
    """
    alice = db.add_node([])
    bob = db.add_node([])
    edge = db.add_edge(alice, "KNOWS", bob)
    db.delete_edge(edge.id)
    assert db.get_edge(edge.id) is None
    assert len(db.out_edges(alice)) == 0


def test_in_edges(db):
    """in_edges() must return all edges whose target is the given node.

    After adding an edge from alice to bob, in_edges(bob) must return exactly
    one edge with the correct label.
    """
    alice = db.add_node([])
    bob = db.add_node([])
    db.add_edge(alice, "KNOWS", bob)
    edges = db.in_edges(bob)
    assert len(edges) == 1
    assert edges[0].label == "KNOWS"


def test_bfs(db):
    """bfs() must discover all reachable nodes within the depth limit.

    Given a chain a -> b -> c and max_depth=2, BFS from a must include both
    b (depth 1) and c (depth 2) in the result list.
    """
    a = db.add_node([])
    b = db.add_node([])
    c = db.add_node([])
    db.add_edge(a, "LINK", b)
    db.add_edge(b, "LINK", c)
    results = db.bfs(a, max_depth=2)
    ids = [node.id for node, depth in results]
    assert b.id in ids
    assert c.id in ids


def test_bfs_max_depth(db):
    """bfs() must not return nodes beyond the specified max_depth.

    Given the chain a -> b -> c, BFS from a with max_depth=1 must include b
    but must exclude c, since c is at depth 2.
    """
    a = db.add_node([])
    b = db.add_node([])
    c = db.add_node([])
    db.add_edge(a, "L", b)
    db.add_edge(b, "L", c)
    results = db.bfs(a, max_depth=1)
    ids = [node.id for node, _ in results]
    assert b.id in ids
    # c is at depth 2 and must be excluded when max_depth=1.
    assert c.id not in ids


def test_dfs(db):
    """dfs() must discover all reachable nodes within the depth limit.

    Given a chain a -> b -> c and max_depth=2, DFS from a must include both
    b (depth 1) and c (depth 2) in the result list. The start node itself is
    not included (matches the documented contract in liel.pyi).
    """
    a = db.add_node([])
    b = db.add_node([])
    c = db.add_node([])
    db.add_edge(a, "LINK", b)
    db.add_edge(b, "LINK", c)
    results = db.dfs(a, max_depth=2)
    ids = [node.id for node, _depth in results]
    assert b.id in ids
    assert c.id in ids
    assert a.id not in ids


def test_dfs_max_depth(db):
    """dfs() must respect max_depth and exclude nodes beyond it.

    Given the chain a -> b -> c, DFS from a with max_depth=1 must include b
    but exclude c. This mirrors the BFS depth contract so the two traversals
    can be swapped by the caller without surprise.
    """
    a = db.add_node([])
    b = db.add_node([])
    c = db.add_node([])
    db.add_edge(a, "L", b)
    db.add_edge(b, "L", c)
    results = db.dfs(a, max_depth=1)
    ids = [node.id for node, _ in results]
    assert b.id in ids
    assert c.id not in ids


def test_dfs_branching(db):
    """dfs() on a branching graph must return every reachable node exactly once.

    From root with two children, both children must appear in the result, and
    no duplicates even when several edges happen to share endpoints.
    """
    root = db.add_node([])
    left = db.add_node([])
    right = db.add_node([])
    db.add_edge(root, "L", left)
    db.add_edge(root, "L", right)
    results = db.dfs(root, max_depth=3)
    ids = [node.id for node, _ in results]
    assert left.id in ids
    assert right.id in ids
    assert len(ids) == len(set(ids)), "DFS must not return duplicate nodes"


def test_node_count(db):
    """node_count() must return the number of nodes currently in the database.

    After adding two nodes, node_count() must return exactly 2.
    """
    db.add_node([])
    db.add_node([])
    assert db.node_count() == 2


def test_edge_count(db):
    """edge_count() must return the total number of edges in the database.

    After adding two edges between the same pair of nodes (multigraph), the
    count must be 2.
    """
    a = db.add_node([])
    b = db.add_node([])
    db.add_edge(a, "L", b)
    db.add_edge(a, "L", b)
    assert db.edge_count() == 2


def test_all_nodes(db):
    """all_nodes() must return a list containing every live node.

    After adding two nodes, all_nodes() must return a list of length 2.
    """
    db.add_node([], name="A")
    db.add_node([], name="B")
    nodes = list(db.all_nodes())
    assert len(nodes) == 2


def test_all_edges(db):
    """all_edges() must return a list containing every live edge.

    After adding one edge, all_edges() must return a list of length 1.
    """
    a = db.add_node([])
    b = db.add_node([])
    db.add_edge(a, "L", b)
    edges = list(db.all_edges())
    assert len(edges) == 1
