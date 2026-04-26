"""liel — PEP 561 type stub for the liel embedded property graph database."""

from __future__ import annotations

from typing import Any, Callable, Literal

__version__: str

# ─── Exceptions ───────────────────────────────────────────────────────────────

class GraphDBError(Exception):
    """Base exception for graph-level liel errors (missing ids, bad on-disk data, invalid transactions).

    File I/O failures from the OS are usually raised as :exc:`OSError` instead,
    so code that opens paths on disk should often catch both.
    """

    ...

class NodeNotFoundError(GraphDBError):
    """Raised when a node id is not present (never created, already deleted, or invalid)."""

    ...

class EdgeNotFoundError(GraphDBError):
    """Raised when an edge id is not present (never created, already deleted, or invalid)."""

    ...

class CorruptedFileError(GraphDBError):
    """Raised when the database fails an internal integrity check and may be damaged.

    The error text is intentionally actionable. For adjacency-list corruption it
    points at the damaged node/edge pair and recommends stopping writes,
    taking a backup, and running :meth:`GraphDB.repair_adjacency` when
    available.
    """

    ...

class TransactionError(GraphDBError):
    """Raised when commit/rollback or transaction nesting rules are violated."""

    ...

class MergeError(GraphDBError):
    """Raised by :meth:`GraphDB.merge_from` when the requested merge cannot be
    completed — for example, when a source node is missing a property listed in
    ``node_key``."""

    ...

class CapacityExceededError(GraphDBError):
    """Raised when an internal capacity limit of the database file format would be
    exceeded by the requested write — for example, when the property payload of a
    single node or edge would not fit in a single WAL entry, or when the number of
    extents per kind reaches its hard cap.

    This usually indicates that a single record is unreasonably large rather than
    that the database itself is full; consider splitting the value or storing the
    blob outside the graph.
    """

    ...

class AlreadyOpenError(GraphDBError):
    """Raised by :func:`open` when another live :class:`GraphDB` handle in the same
    Python process is already attached to the requested ``.liel`` file.

    liel guarantees crash-safety only with **a single writer per file**. Opening
    the same path twice from one process would let two independent pagers commit
    against the same WAL, silently clobbering one another. Close the existing
    handle (or its ``with`` block) before re-opening, or run the second consumer
    in a separate process if it really needs concurrent read access.
    """

    ...

# ─── Data classes ─────────────────────────────────────────────────────────────

class Node:
    """Represents a node (vertex) in the property graph.

    Nodes have a stable integer ID, a list of string labels, and a dictionary
    of typed properties (bool, int, float, str, list, or dict values).
    """

    @property
    def id(self) -> int:
        """The unique 1-based integer ID assigned to this node at creation time."""
        ...

    @property
    def labels(self) -> list[str]:
        """The list of string labels attached to this node (e.g. ["Person"])."""
        ...

    @property
    def properties(self) -> dict[str, Any]:
        """A dictionary copy of all properties stored on this node."""
        ...

    def __getitem__(self, key: str) -> Any:
        """Return the value of the property named *key*.

        Raises KeyError if the property does not exist.
        """
        ...

    def __contains__(self, key: str) -> bool:
        """Return True if a property named *key* exists on this node."""
        ...

    def get(self, key: str) -> Any | None:
        """Return the value of property *key*, or None if it is absent."""
        ...

    def keys(self) -> list[str]:
        """Return the list of all property names stored on this node."""
        ...

    def __repr__(self) -> str: ...

class Edge:
    """Represents a directed edge (relationship) between two nodes.

    Every edge has a unique integer ID, a single string label, references to
    its source and target node IDs, and an optional dictionary of typed
    properties.
    """

    @property
    def id(self) -> int:
        """The unique 1-based integer ID assigned to this edge at creation time."""
        ...

    @property
    def label(self) -> str:
        """The relationship type label for this edge (e.g. "KNOWS")."""
        ...

    @property
    def from_node(self) -> int:
        """The ID of the source node (tail) of this directed edge."""
        ...

    @property
    def to_node(self) -> int:
        """The ID of the target node (head) of this directed edge."""
        ...

    @property
    def properties(self) -> dict[str, Any]:
        """A dictionary copy of all properties stored on this edge."""
        ...

    def __getitem__(self, key: str) -> Any:
        """Return the value of the property named *key*.

        Raises KeyError if the property does not exist.
        """
        ...

    def __contains__(self, key: str) -> bool:
        """Return True if a property named *key* exists on this edge."""
        ...

    def get(self, key: str) -> Any | None:
        """Return the value of property *key*, or None if it is absent."""
        ...

    def keys(self) -> list[str]:
        """Return the list of all property names stored on this edge."""
        ...

    def __repr__(self) -> str: ...

# ─── Query builder ────────────────────────────────────────────────────────────

class NodeQuery:
    """A lazy, chainable query builder for filtering and paginating nodes.

    Obtain an instance via ``db.nodes()``.  Each method returns a new
    NodeQuery so that the original query object is never mutated.

    Example::

        adults = db.nodes().label("Person").where_(lambda n: n["age"] >= 18).fetch()
    """

    def label(self, label: str) -> NodeQuery:
        """Restrict results to nodes that carry the given label string."""
        ...

    def where_(self, predicate: Callable[[Node], bool]) -> NodeQuery:
        """Restrict results to nodes for which *predicate* returns True."""
        ...

    def skip(self, n: int) -> NodeQuery:
        """Skip the first *n* matching nodes (offset-based pagination)."""
        ...

    def limit(self, n: int) -> NodeQuery:
        """Return at most *n* matching nodes (limit-based pagination)."""
        ...

    def fetch(self) -> list[Node]:
        """Execute the query and return all matching nodes as a list."""
        ...

    def count(self) -> int:
        """Execute the query and return the number of matching nodes."""
        ...

    def exists(self) -> bool:
        """Return True if at least one node matches the current query.

        Short-circuits after the first surviving node: the scan stops as soon
        as one node passes the label filter, the ``where_`` predicate, and
        any ``skip``.  Any ``limit`` set on the query is overridden to 1
        because a smaller cap cannot change the answer.
        """
        ...

class EdgeQuery:
    """A lazy, chainable query builder for filtering and paginating edges.

    Obtain an instance via ``db.edges()``.  Each method returns a new
    EdgeQuery so that the original query object is never mutated.

    Example::

        recent = db.edges().label("KNOWS").where_(lambda e: e["since"] >= 2020).fetch()
    """

    def label(self, label: str) -> EdgeQuery:
        """Restrict results to edges whose label matches the given string."""
        ...

    def where_(self, predicate: Callable[[Edge], bool]) -> EdgeQuery:
        """Restrict results to edges for which *predicate* returns True."""
        ...

    def skip(self, n: int) -> EdgeQuery:
        """Skip the first *n* matching edges (offset-based pagination)."""
        ...

    def limit(self, n: int) -> EdgeQuery:
        """Return at most *n* matching edges (limit-based pagination)."""
        ...

    def fetch(self) -> list[Edge]:
        """Execute the query and return all matching edges as a list."""
        ...

    def count(self) -> int:
        """Execute the query and return the number of matching edges."""
        ...

    def exists(self) -> bool:
        """Return True if at least one edge matches the current query.

        Short-circuits after the first surviving edge (same semantics as
        :meth:`NodeQuery.exists`): any ``limit`` set on the query is
        overridden to 1.
        """
        ...

# ─── Transaction ──────────────────────────────────────────────────────────────

class MergeReport:
    """Summary of a :meth:`GraphDB.merge_from` call.

    All counters are non-negative.  ``node_id_map`` maps every source node ID
    to its destination ID (whether freshly created or reused via
    ``node_key``).  ``edge_id_map`` maps every source edge ID to the resulting
    destination edge ID (new or, for ``edge_strategy="idempotent"``, reused).
    """

    @property
    def node_id_map(self) -> dict[int, int]:
        """Source node ID → destination node ID for every source node processed."""
        ...

    @property
    def edge_id_map(self) -> dict[int, int]:
        """Source edge ID → destination edge ID for every source edge processed."""
        ...

    @property
    def nodes_created(self) -> int:
        """Number of destination nodes freshly inserted during the merge."""
        ...

    @property
    def nodes_reused(self) -> int:
        """Number of source nodes matched to an existing destination node via ``node_key``."""
        ...

    @property
    def edges_created(self) -> int:
        """Number of destination edges freshly inserted during the merge."""
        ...

    @property
    def edges_reused(self) -> int:
        """Number of source edges deduplicated onto an existing destination edge.

        Always zero when ``edge_strategy="append"``.
        """
        ...

    def __repr__(self) -> str: ...

class Transaction:
    """Context manager that wraps a database transaction.

    Obtained via ``db.transaction()``.  On normal exit the transaction is
    automatically committed.  If an exception propagates out of the ``with``
    block the transaction is automatically rolled back and the exception
    continues to propagate.

    Nesting is forbidden: entering a second ``with db.transaction()`` block
    while one is already active raises :class:`TransactionError`.  The
    explicit-transaction flag is toggled by ``__enter__`` / ``__exit__``,
    so a stray ``db.transaction()`` that is never entered does not block
    future calls.  Calling :meth:`GraphDB.vacuum` inside an explicit
    transaction also raises :class:`TransactionError` because vacuum
    forces an internal commit.
    """

    def __enter__(self) -> Transaction: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: Any | None,
    ) -> bool:
        """Commit on success or roll back on exception; always re-raises."""
        ...

# ─── GraphDB ──────────────────────────────────────────────────────────────────

class GraphDB:
    """The main handle to a liel embedded property graph database file.

    Obtain an instance with ``liel.open(path)``.  A transaction is implicitly
    started when the database is opened; call ``commit()`` to persist changes,
    or ``rollback()`` to discard them.  The file is closed with ``close()`` or
    by using the database as a context manager (``with liel.open(...) as db``).

    Writes to the same ``.liel`` file must be centralized through one owner
    process. liel rejects dangerous double-opens with an in-process registry
    and a cross-process ``<file>.lock/`` directory; this is fail-closed
    protection, not multi-writer support.
    """

    # ── Lifecycle ────────────────────────────────────────────────────────────

    def close(self) -> None:
        """Close this handle and release its in-process writer slot.

        Pending changes are not committed automatically; call ``commit()``
        first if they should persist. After ``close()``, further operations on
        this handle raise ``GraphDBError``.
        """
        ...

    def __enter__(self) -> GraphDB: ...
    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> bool:
        """Exit the ``with`` block, close the handle, and propagate exceptions.

        The block does not auto-commit. Use ``db.transaction()`` for
        commit-on-success / rollback-on-exception semantics.
        """
        ...

    # ── Node operations ──────────────────────────────────────────────────────

    def add_node(self, labels: list[str] = ..., **properties: Any) -> Node:
        """Create a new node and return it.

        Args:
            labels: Zero or more string labels to attach to the node.
            **properties: Keyword arguments are stored as node properties.
                          Supported value types: None, bool, int, float, str,
                          list, and dict (values may nest recursively).

        Returns:
            The newly created Node with a 1-based integer ID.
        """
        ...

    def get_node(self, node_id: int) -> Node | None:
        """Fetch the node with the given ID, or return None if not found."""
        ...

    def update_node(self, node: int | Node, **properties: Any) -> None:
        """Replace all properties on the node.

        Accepts either a Node object or a raw integer node ID.
        The node's labels are not changed.  Raises NodeNotFoundError if the
        node does not exist.
        """
        ...

    def delete_node(self, node: int | Node) -> None:
        """Delete the node (and all its incident edges) from the database.

        Accepts either a Node object or a raw integer node ID.
        Raises NodeNotFoundError if the node does not exist.
        """
        ...

    # ── Edge operations ──────────────────────────────────────────────────────

    def add_edge(
        self, from_node: int | Node, label: str, to_node: int | Node, **properties: Any
    ) -> Edge:
        """Create a new directed edge from *from_node* to *to_node*.

        Args:
            from_node: Source node (Node object or integer ID).
            label: Relationship type string (e.g. "KNOWS").
            to_node: Target node (Node object or integer ID).
            **properties: Edge property key/value pairs.

        Returns:
            The newly created Edge.

        Raises:
            NodeNotFoundError: If either endpoint node ID does not exist.
        """
        ...

    def get_edge(self, edge_id: int) -> Edge | None:
        """Fetch the edge with the given ID, or return None if not found."""
        ...

    def update_edge(self, edge_id: int, **properties: Any) -> None:
        """Replace all properties on the edge with *edge_id*.

        Raises EdgeNotFoundError if the edge does not exist.
        """
        ...

    def delete_edge(self, edge: int | Edge) -> None:
        """Delete the edge from the database.

        Accepts either an Edge object or a raw integer edge ID.
        Raises EdgeNotFoundError if the edge does not exist.
        """
        ...

    def merge_edge(
        self, from_node: int | Node, label: str, to_node: int | Node, **properties: Any
    ) -> Edge:
        """Return an existing edge matching (from, label, to, properties) or create one.

        If an edge with the same endpoints, label, and identical properties
        already exists it is returned unchanged.  If the properties differ a
        new edge is created.  This is useful for idempotent graph construction.
        """
        ...

    # ── Adjacency queries ────────────────────────────────────────────────────

    def out_edges(self, node: int | Node, label: str | None = None) -> list[Edge]:
        """Return all edges that originate from *node*.

        Args:
            node: Source node (Node object or integer ID).
            label: If given, only edges with this label are returned.

        Order:
            Edges are returned in **reverse insertion order** (head-insert
            singly-linked list on disk).  The order is stable for a given
            graph state but is not sorted by value, ID, or label — sort the
            result yourself if you need a specific order.
        """
        ...

    def in_edges(self, node: int | Node, label: str | None = None) -> list[Edge]:
        """Return all edges that terminate at *node*.

        Args:
            node: Target node (Node object or integer ID).
            label: If given, only edges with this label are returned.

        Order:
            Same contract as :meth:`out_edges` — reverse insertion order,
            stable but not sorted.
        """
        ...

    def neighbors(
        self,
        node: int | Node,
        edge_label: str | None = None,
        direction: str = "out",  # "out" | "in" | "both"
    ) -> list[Node]:
        """Return the neighbor nodes reachable from *node* via the given direction.

        Args:
            node: The starting node (Node object or integer ID).
            edge_label: If given, traverse only edges with this label.
            direction: One of ``"out"`` (default), ``"in"``, or ``"both"``.

        Returns:
            A list of neighbor Nodes (duplicates possible when direction="both"
            and self-loops exist).

        Order:
            Follows the same reverse-insertion order as :meth:`out_edges` /
            :meth:`in_edges`.  Stable but not sorted.

        Raises:
            ValueError: If *direction* is not a recognised value.
        """
        ...

    # ── Graph traversal ──────────────────────────────────────────────────────

    def bfs(self, start: int | Node, max_depth: int) -> list[tuple[Node, int]]:
        """Breadth-first search starting at *start*, up to *max_depth* hops.

        Returns:
            A list of (Node, depth) tuples in BFS discovery order.  The start
            node itself is not included.  Within a single depth layer, nodes
            appear in the order their incoming edge was discovered, which
            follows :meth:`out_edges`' reverse-insertion order.
        """
        ...

    def dfs(self, start: int | Node, max_depth: int) -> list[tuple[Node, int]]:
        """Depth-first search starting at *start*, up to *max_depth* hops.

        Returns:
            A list of (Node, depth) tuples in DFS discovery order.  The start
            node itself is not included.  Branch order follows
            :meth:`out_edges`' reverse-insertion order.
        """
        ...

    def shortest_path(
        self,
        start: int | Node,
        goal: int | Node,
        edge_label: str | None = None,
    ) -> list[Node] | None:
        """Return the shortest directed path between *start* and *goal*.

        Args:
            start: The source node.
            goal: The target node.
            edge_label: If given, only traverse outgoing edges with this label.

        Returns:
            An ordered list of Nodes from *start* to *goal* (inclusive), or
            None if no path exists along outgoing edges.
        """
        ...

    # ── Full-scan accessors ──────────────────────────────────────────────────

    def all_nodes(self) -> list[Node]:
        """Return every non-deleted node currently in the database."""
        ...

    def all_edges(self) -> list[Edge]:
        """Return every non-deleted edge currently in the database."""
        ...

    def all_nodes_as_records(self) -> list[dict[str, Any]]:
        """Return all nodes as plain Python dicts in a single boundary crossing.

        Each dict contains ``"id"`` (int), ``"labels"`` (list[str]), and one
        key per node property.  Prefer this over ``all_nodes()`` when building
        DataFrames or passing data to NetworkX, as it avoids per-node PyO3
        wrapper allocation.
        """
        ...

    def all_edges_as_records(self) -> list[dict[str, Any]]:
        """Return all edges as plain Python dicts in a single boundary crossing.

        Each dict contains ``"id"``, ``"label"``, ``"from_node"``,
        ``"to_node"``, and one key per edge property.
        """
        ...

    def degree_stats(self) -> dict[int, tuple[int, int]]:
        """Return ``{node_id: (out_degree, in_degree)}`` computed in Rust.

        Only nodes that appear as an edge endpoint are included.  Nodes with no
        edges are absent from the result (treat missing entries as ``(0, 0)``).
        """
        ...

    def edges_between(self, node_ids: Any) -> list[dict[str, Any]]:
        """Return edge records whose both endpoints are in *node_ids*.

        *node_ids* may be any Python iterable of integer node IDs (``set``,
        ``list``, generator, …).  Filtering is done in Rust; no Python loop
        over the full edge set is required.

        Each returned dict has the same shape as ``all_edges_as_records()``.
        """
        ...

    def node_count(self) -> int:
        """Return the current number of live (non-deleted) nodes.

        May raise ``RuntimeError`` if the internal database lock was poisoned.
        """
        ...

    def edge_count(self) -> int:
        """Return the current number of live (non-deleted) edges.

        May raise ``RuntimeError`` if the internal database lock was poisoned.
        """
        ...

    # ── Query builder entry points ───────────────────────────────────────────

    def nodes(self) -> NodeQuery:
        """Return a NodeQuery builder for filtering and paginating all nodes."""
        ...

    def edges(self) -> EdgeQuery:
        """Return an EdgeQuery builder for filtering and paginating all edges."""
        ...

    # ── Transaction control ──────────────────────────────────────────────────

    def begin(self) -> None:
        """Explicitly begin a new transaction (no-op if one is already active).

        A transaction is started implicitly on ``open()``, so calling this is
        usually unnecessary.
        """
        ...

    def commit(self) -> None:
        """Persist all changes made since the last commit (or since open).

        Writes the WAL to disk and then checkpoints it into the main data
        pages, following the WAL-before-data ordering guarantee.
        """
        ...

    def rollback(self) -> None:
        """Discard all changes made since the last commit (or since open).

        The database is returned to the state it was in at the start of the
        current transaction.
        """
        ...

    def transaction(self) -> Transaction:
        """Return a context manager that auto-commits or auto-rolls back.

        Usage::

            with db.transaction():
                db.add_node(["Person"], name="Alice")
            # committed automatically

        If an exception propagates out of the block the transaction is rolled
        back and the exception is re-raised.
        """
        ...

    # ── Cross-database merge ─────────────────────────────────────────────────

    def merge_from(
        self,
        other: GraphDB,
        *,
        node_key: list[str] | None = None,
        edge_strategy: Literal["append", "idempotent"] = "append",
        on_node_conflict: Literal["keep_dst", "overwrite_from_src", "merge_props"] = "keep_dst",
    ) -> MergeReport:
        """Merge every live node and edge from *other* into this database.

        Node IDs and edge IDs from *other* are always remapped because each
        ``.liel`` file allocates IDs from its own independent counter — two
        databases can both contain ``NodeId(5)`` that refer to unrelated
        entities.  The returned :class:`MergeReport` exposes the source → dest
        ID maps that were used for the rewrite.

        The source database is read-only during the call; it is never mutated.
        Writes to ``self`` happen inside the implicit transaction, so wrap the
        call in ``with self.transaction():`` (or call ``self.commit()``
        explicitly) to persist them.

        Args:
            other: A different :class:`GraphDB` instance.  Passing the same
                database as ``self`` raises :class:`ValueError`.
            node_key: If given, source nodes are considered to match an
                existing destination node whenever **all** of the listed
                property keys have equal values on both sides.  If ``None``
                (default), every source node becomes a fresh destination node.
                A source node that lacks any requested key raises
                :class:`MergeError`.
            edge_strategy: ``"append"`` (default) always calls
                :meth:`add_edge`, preserving duplicates.  ``"idempotent"``
                calls :meth:`merge_edge`, reusing an existing destination
                edge when ``(from, label, to, properties)`` matches exactly.
            on_node_conflict: Only relevant when ``node_key`` is given and a
                source node matches an existing destination node.
                ``"keep_dst"`` (default) leaves the destination untouched,
                discarding the source's properties.  ``"overwrite_from_src"``
                overlays the source's properties onto the destination (source
                wins on key collision).  ``"merge_props"`` fills in only keys
                that are missing on the destination (destination wins on key
                collision).  Labels are never modified regardless of mode.

        Returns:
            A :class:`MergeReport` with the source → dest ID maps and
            four counters summarising the outcome.

        Raises:
            ValueError: If ``other is self``, or if ``edge_strategy`` /
                ``on_node_conflict`` / empty ``node_key`` values are invalid.
            MergeError: If a source node is missing a property listed in
                ``node_key``.
        """
        ...

    # ── Utility ──────────────────────────────────────────────────────────────

    def vacuum(self) -> None:
        """Compact the property storage section of the file.

        After many updates or deletions, orphaned property blobs accumulate.
        ``vacuum()`` rewrites the property pages, keeping only live property
        data and reducing the file size.
        """
        ...

    def clear(self) -> None:
        """Delete all nodes and edges and reset the ID counters to 1.

        The database file is truncated to its minimum size.  This is
        equivalent to closing and recreating the file.
        """
        ...

    def repair_adjacency(self) -> dict[str, int]:
        """Rebuild node adjacency heads, degree counters, and edge next-pointers.

        This maintenance function treats the live edge set as the source of
        truth and rewrites the adjacency metadata accordingly. It is intended
        for damaged databases where adjacency lists became inconsistent.

        Returns:
            A dict with ``"nodes_rewritten"`` and ``"edges_relinked"`` counts.

        Raises:
            CorruptedFileError: If a live edge points at a missing/deleted node,
                which indicates corruption beyond adjacency links alone.
        """
        ...

    def info(self) -> dict[str, Any]:
        """Return a dictionary of database metadata.

        Guaranteed keys: ``"version"``, ``"node_count"``, ``"edge_count"``,
        ``"file_size"``.  Additional keys may be present in future versions.

        The ``"version"`` string is a **fixed display label** for the on-disk
        format (e.g. ``"1.0"``), not the Python package's ``liel.__version__``
        and not read dynamically from the wheel or disk at runtime.
        """
        ...

# ─── Module-level functions ───────────────────────────────────────────────────

def open(path: str) -> GraphDB:
    """Open (or create) a liel database file and return a GraphDB handle.

    Args:
        path: Filesystem path to the ``.liel`` file, or the special string
              ``":memory:"`` for an in-process, non-persistent database.

    Returns:
        A GraphDB instance with an implicit transaction already started.
        Only one process should perform writes against the same file at a time.
        A second writer process is rejected with ``AlreadyOpenError`` when the
        lock directory can be acquired reliably.

    Raises:
        AlreadyOpenError: If another live :class:`GraphDB` handle or writer
            process is already attached to the same file. Close the existing
            handle, or route all writes through one owner process.
        CorruptedFileError: If *path* points to a file that is not a valid
            liel database (bad magic bytes or checksum).
        OSError: If the file cannot be opened, created, or read (permissions,
            missing parent directory, media errors, etc.).
    """
    ...
