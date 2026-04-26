use super::edge::out_edges;
use super::node::{get_node, Node};
use crate::error::Result;
use crate::storage::pager::Pager;
use std::collections::{HashMap, HashSet, VecDeque};

/// Perform a Breadth-First Search (BFS) starting from `start` up to `max_depth` hops.
///
/// BFS explores the graph level by level, guaranteeing that nodes are visited
/// in non-decreasing distance (hop-count) order from the start node.  It is
/// well-suited to finding all nodes reachable within a bounded radius and is
/// also the foundation of [`shortest_path`].
///
/// ## Algorithm
///
/// State:
/// - `visited: HashSet<u64>` — tracks nodes already enqueued to prevent
///   revisiting them.  This is critical for graphs containing cycles; without
///   it the traversal would loop forever.
/// - `queue: VecDeque<(node_id, depth)>` — the BFS frontier, processed FIFO.
///
/// Loop:
/// 1. Dequeue `(node_id, depth)` from the front of the queue.
/// 2. If `depth > 0`, fetch the full [`Node`] and add it to the result.
///    (The start node is at depth 0 and is intentionally excluded from output.)
/// 3. If `depth >= max_depth`, do not expand this node's neighbours (depth
///    limit reached).
/// 4. Otherwise, fetch all outgoing edges of `node_id` (no label filter).
///    For each unvisited neighbour, mark it visited and enqueue at `depth + 1`.
///
/// ## Complexity
/// O(V + E) where V = reachable nodes and E = reachable edges.
///
/// ## Parameters
/// - `pager`: Mutable reference to the open [`Pager`].
/// - `start`: The node ID to start from. It is included in `visited` but not in the returned results.
/// - `max_depth`: Maximum hops to follow. A value of `0` returns an empty result (depth 0 is excluded from output and not expanded).
///
/// ## Returns
/// A `Vec<(Node, depth)>` where `depth` is the number of edges between `start`
/// and that node.  The start node itself is never included.
///
/// ## Errors
/// Propagates [`crate::error::LielError::Io`] /
/// [`crate::error::LielError::CorruptedFile`] from the pager.
pub fn bfs(pager: &mut Pager, start: u64, max_depth: usize) -> Result<Vec<(Node, usize)>> {
    let mut visited: HashSet<u64> = HashSet::new();
    let mut queue: VecDeque<(u64, usize)> = VecDeque::new();
    let mut result = Vec::new();

    // Seed: mark the start node visited and push it onto the queue at depth 0.
    visited.insert(start);
    queue.push_back((start, 0));

    while let Some((node_id, depth)) = queue.pop_front() {
        // Only add nodes beyond the start (depth > 0) to the result.
        if depth > 0 {
            if let Some(node) = get_node(pager, node_id)? {
                result.push((node, depth));
            }
        }
        // Do not expand neighbours if we have reached the depth limit.
        if depth >= max_depth {
            continue;
        }
        // Expand: enqueue all unvisited out-neighbours at depth + 1.
        let edges = out_edges(pager, node_id, None)?;
        for edge in edges {
            let neighbor = edge.to;
            if !visited.contains(&neighbor) {
                // Mark visited *before* enqueuing to prevent duplicates even
                // when multiple paths reach the same node at the same depth.
                visited.insert(neighbor);
                queue.push_back((neighbor, depth + 1));
            }
        }
    }
    Ok(result)
}

/// Perform a Depth-First Search (DFS) starting from `start` up to `max_depth` hops.
///
/// DFS explores the graph by going as deep as possible along each branch before
/// backtracking.  Compared to BFS, DFS uses a LIFO stack instead of a FIFO
/// queue, which means nodes closer to the start may appear later in the output.
/// Use BFS when you need nodes in distance order; use DFS when you want to
/// enumerate paths or perform post-order processing.
///
/// ## Algorithm
///
/// State:
/// - `visited: HashSet<u64>` — cycle-prevention guard (same role as in BFS).
/// - `stack: Vec<(node_id, depth)>` — the DFS frontier, processed LIFO.
///
/// Loop:
/// 1. Pop `(node_id, depth)` from the top of the stack.
/// 2. If `depth > 0`, fetch the full [`Node`] and add it to the result.
/// 3. If `depth >= max_depth`, skip expansion.
/// 4. Fetch outgoing edges and push unvisited neighbours onto the stack at
///    `depth + 1`.
///
/// Because the stack is LIFO and edges are pushed in the order returned by
/// [`out_edges`] (which uses head-insert, i.e. reverse-insertion order), the
/// DFS exploration order depends on the insertion history of edges.
///
/// ## Parameters
/// - `pager`:     Mutable reference to the open [`Pager`].
/// - `start`:     Starting node ID; excluded from output (same as BFS).
/// - `max_depth`: Maximum hop depth; expansion stops at this limit.
///
/// ## Returns
/// A `Vec<(Node, depth)>` in DFS visit order.  The start node is excluded.
///
/// ## Errors
/// Propagates [`crate::error::LielError::Io`] /
/// [`crate::error::LielError::CorruptedFile`] from the pager.
pub fn dfs(pager: &mut Pager, start: u64, max_depth: usize) -> Result<Vec<(Node, usize)>> {
    let mut visited: HashSet<u64> = HashSet::new();
    let mut stack: Vec<(u64, usize)> = Vec::new();
    let mut result = Vec::new();

    // Seed: mark the start node visited and push onto the stack at depth 0.
    visited.insert(start);
    stack.push((start, 0));

    while let Some((node_id, depth)) = stack.pop() {
        // Collect the node for the result (skip the start at depth 0).
        if depth > 0 {
            if let Some(node) = get_node(pager, node_id)? {
                result.push((node, depth));
            }
        }
        // Depth limit: don't explore further from this node.
        if depth >= max_depth {
            continue;
        }
        // Push unvisited out-neighbours onto the stack for later processing.
        let edges = out_edges(pager, node_id, None)?;
        for edge in edges {
            let neighbor = edge.to;
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                stack.push((neighbor, depth + 1));
            }
        }
    }
    Ok(result)
}

/// Find the shortest path between two nodes using BFS with parent tracking.
///
/// Returns the minimum-hop path from `start` to `goal` as an ordered list of
/// [`Node`] values, or `None` if no path exists.  The returned path includes
/// both the `start` and `goal` nodes.
///
/// ## Why BFS gives the shortest path
/// BFS explores nodes in non-decreasing distance order.  The first time the
/// goal node is discovered, it was reached via the minimum number of hops.
/// DFS does not have this property and would require exhaustive search with
/// backtracking to find the true shortest path.
///
/// ## Algorithm
///
/// State:
/// - `visited: HashSet<u64>` — prevents revisiting nodes.
/// - `queue: VecDeque<u64>` — BFS frontier (node IDs only, no depth needed).
/// - `parent: HashMap<u64, u64>` — maps each discovered node to the node from
///   which it was first reached.  Used to reconstruct the path once the goal
///   is found.
///
/// Loop:
/// 1. Dequeue `node_id`.
/// 2. Fetch outgoing edges (optionally filtered by `edge_label`).
/// 3. For each unvisited neighbour:
///    a. Mark visited.
///    b. Record `parent[neighbour] = node_id`.
///    c. If `neighbour == goal`, reconstruct the path and return it.
///    d. Otherwise enqueue `neighbour`.
///
/// ## Path reconstruction
/// Starting from `goal`, follow the `parent` chain back to `start` (the only
/// node with no parent entry), collecting node IDs into a `Vec`.  Reverse the
/// Vec to get the path from start to goal, then hydrate each ID into a full
/// [`Node`] by calling [`get_node`].
///
/// ## Edge cases
/// - `start == goal`: Returns a single-element path `[start_node]` immediately.
/// - `start` does not exist: [`get_node`] returns `None`, so the path Vec will
///   be missing the start node (the caller should validate IDs beforehand).
///
/// ## Parameters
/// - `pager`:      Mutable reference to the open [`Pager`].
/// - `start`:      Source node ID.
/// - `goal`:       Target node ID.
/// - `edge_label`: If `Some(label)`, only edges with that label are traversed.
///   If `None`, all outgoing edges are used.
///
/// ## Returns
/// - `Ok(Some(Vec<Node>))` — the shortest path including both endpoints.
/// - `Ok(None)` — no path exists between `start` and `goal` under the given
///   edge-label constraint.
///
/// ## Errors
/// Propagates [`crate::error::LielError::Io`] /
/// [`crate::error::LielError::CorruptedFile`] from the pager.
pub fn shortest_path(
    pager: &mut Pager,
    start: u64,
    goal: u64,
    edge_label: Option<&str>,
) -> Result<Option<Vec<Node>>> {
    // Trivial case: start and goal are the same node.
    if start == goal {
        if let Some(node) = get_node(pager, start)? {
            return Ok(Some(vec![node]));
        }
        return Ok(None);
    }

    let mut visited: HashSet<u64> = HashSet::new();
    let mut queue: VecDeque<u64> = VecDeque::new();
    // parent[child] = the node through which child was first discovered.
    // Used to walk backwards from the goal to the start during reconstruction.
    let mut parent: HashMap<u64, u64> = HashMap::new();

    visited.insert(start);
    queue.push_back(start);

    while let Some(node_id) = queue.pop_front() {
        // Fetch outgoing edges, applying the optional label filter.
        let edges = if let Some(label) = edge_label {
            out_edges(pager, node_id, Some(label))?
        } else {
            out_edges(pager, node_id, None)?
        };

        for edge in edges {
            let neighbor = edge.to;
            if visited.contains(&neighbor) {
                // Already discovered via a shorter or equal-length path;
                // skip to avoid overwriting the parent entry.
                continue;
            }
            visited.insert(neighbor);
            // Record how we reached this neighbour.
            parent.insert(neighbor, node_id);

            if neighbor == goal {
                // Goal found — reconstruct the path by walking parent pointers
                // from `goal` back to `start`.
                let mut path_ids = Vec::new();
                let mut current = goal;
                loop {
                    path_ids.push(current);
                    match parent.get(&current) {
                        Some(&p) => current = p,
                        // We've reached the start node, which has no parent entry.
                        None => break,
                    }
                }
                // path_ids is [goal, ..., start]; reverse to get [start, ..., goal].
                path_ids.reverse();

                // Hydrate IDs into full Node objects.
                let mut path_nodes = Vec::new();
                for id in path_ids {
                    if let Some(node) = get_node(pager, id)? {
                        path_nodes.push(node);
                    }
                }
                return Ok(Some(path_nodes));
            }
            // Goal not yet found; continue BFS from this neighbour.
            queue.push_back(neighbor);
        }
    }
    // Queue exhausted without reaching the goal.
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::edge::add_edge;
    use crate::graph::node::add_node;
    use crate::storage::pager::Pager;
    use std::collections::HashMap;

    fn make_db() -> Pager {
        Pager::open(":memory:").unwrap()
    }

    #[test]
    fn test_bfs_single_hop() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), c.id, HashMap::new()).unwrap();
        let result = bfs(&mut pager, a.id, 1).unwrap();
        let ids: Vec<u64> = result.iter().map(|(n, _)| n.id).collect();
        assert!(ids.contains(&b.id));
        assert!(ids.contains(&c.id));
    }

    #[test]
    fn test_bfs_two_hops() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, b.id, "E".into(), c.id, HashMap::new()).unwrap();
        let result = bfs(&mut pager, a.id, 2).unwrap();
        let ids: Vec<u64> = result.iter().map(|(n, _)| n.id).collect();
        assert!(ids.contains(&b.id));
        assert!(ids.contains(&c.id));
    }

    #[test]
    fn test_bfs_max_depth_cutoff() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, b.id, "E".into(), c.id, HashMap::new()).unwrap();
        let result = bfs(&mut pager, a.id, 1).unwrap();
        let ids: Vec<u64> = result.iter().map(|(n, _)| n.id).collect();
        assert!(ids.contains(&b.id));
        assert!(!ids.contains(&c.id));
    }

    #[test]
    fn test_bfs_cycle_safe() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, b.id, "E".into(), a.id, HashMap::new()).unwrap();
        let result = bfs(&mut pager, a.id, 10).unwrap();
        // The visited set prevents infinite looping on this cycle.
        let ids: Vec<u64> = result.iter().map(|(n, _)| n.id).collect();
        assert!(ids.contains(&b.id));
    }

    #[test]
    fn test_bfs_disconnected() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap(); // isolated node, no edges
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let result = bfs(&mut pager, a.id, 10).unwrap();
        let ids: Vec<u64> = result.iter().map(|(n, _)| n.id).collect();
        assert!(!ids.contains(&c.id));
    }

    #[test]
    fn test_dfs_simple() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, b.id, "E".into(), c.id, HashMap::new()).unwrap();
        let result = dfs(&mut pager, a.id, 5).unwrap();
        let ids: Vec<u64> = result.iter().map(|(n, _)| n.id).collect();
        assert!(ids.contains(&b.id));
        assert!(ids.contains(&c.id));
    }

    #[test]
    fn test_dfs_cycle_safe() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, b.id, "E".into(), a.id, HashMap::new()).unwrap();
        let result = dfs(&mut pager, a.id, 10).unwrap();
        // visited set prevents infinite looping on this back-edge cycle.
        let ids: Vec<u64> = result.iter().map(|(n, _)| n.id).collect();
        assert!(ids.contains(&b.id));
    }

    #[test]
    fn test_shortest_path_direct() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        let path = shortest_path(&mut pager, a.id, b.id, None).unwrap();
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].id, a.id);
        assert_eq!(path[1].id, b.id);
    }

    #[test]
    fn test_shortest_path_indirect() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "E".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, b.id, "E".into(), c.id, HashMap::new()).unwrap();
        let path = shortest_path(&mut pager, a.id, c.id, None).unwrap();
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn test_shortest_path_none() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        // No edges between a and b — no path exists.
        let path = shortest_path(&mut pager, a.id, b.id, None).unwrap();
        assert!(path.is_none());
    }

    #[test]
    fn test_shortest_path_with_label_filter() {
        let mut pager = make_db();
        let a = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let b = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        let c = add_node(&mut pager, vec![], HashMap::new()).unwrap();
        add_edge(&mut pager, a.id, "KNOWS".into(), b.id, HashMap::new()).unwrap();
        add_edge(&mut pager, b.id, "LIKES".into(), c.id, HashMap::new()).unwrap();
        // Traversing only "KNOWS" edges cannot reach c (the a→b→c path requires
        // a "LIKES" edge for the second hop).
        let path = shortest_path(&mut pager, a.id, c.id, Some("KNOWS")).unwrap();
        assert!(path.is_none());
        // Without a label filter all edges are usable, so c is reachable.
        let path = shortest_path(&mut pager, a.id, c.id, None).unwrap();
        assert!(path.is_some());
    }
}
