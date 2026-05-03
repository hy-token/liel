"""Minimal CodingMemory graph using ``liel.coding_memory`` helpers."""

from __future__ import annotations

import liel
from liel.coding_memory import link, record_bug, record_decision, record_file


def main() -> None:
    db = liel.open(":memory:")
    with db.transaction():
        py = record_file(db, "python/liel/coding_memory.py", role="helper")
        dec = record_decision(
            db,
            "Ship CodingMemory helpers before LangGraph adapter samples",
            status="draft",
        )
        bug = record_bug(db, "Duplicate path handling undocumented", severity="S3")
        link(db, py, "RELATES_TO", dec)
        link(db, bug, "DEPENDS_ON", py)
    db.commit()

    print("nodes:", db.node_count(), "edges:", db.edge_count())
    for row in db.all_nodes_as_records():
        print(row)
    db.close()


if __name__ == "__main__":
    main()
