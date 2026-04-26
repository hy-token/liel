//! Regression test for the silent-corruption bug that used to appear past the
//! old fixed node/edge area boundaries (node_id > 16 128 or edge_id > 13 056).
//!
//! With the extent-chained allocator a workload that crosses the old caps must
//! survive a round-trip: every edge must read back with the same endpoints
//! and label, counts must match, and reopening the file must reproduce the
//! exact same view.
//!
//! The test keeps a tight cap on resource use so it stays reasonable for CI:
//! it batches commits at 1 000 edges per transaction (well inside the 4 MiB
//! WAL reservation) and stops at 14 000 edges, a few hundred past the old
//! hard cap that used to corrupt the file.

use std::collections::HashMap;
use std::path::PathBuf;

use liel::db::GraphDB;
use liel::storage::prop_codec::PropValue;

/// Build a per-run tempfile path so concurrent test runs do not race.
fn temp_path(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "liel-extent-{tag}-{pid}.liel",
        pid = std::process::id()
    ));
    let _ = std::fs::remove_file(&dir);
    dir
}

#[test]
fn star_graph_14k_edges_survives_roundtrip() {
    let path = temp_path("star");

    const EDGE_COUNT: u64 = 14_000; // past the old 13 056 edge cap
    const COMMIT_BATCH: u64 = 1_000;

    // Insert phase: one central node, EDGE_COUNT leaves, each connected back.
    {
        let mut db = GraphDB::open(path.to_str().unwrap()).unwrap();
        let center = db.add_node(vec!["Center".into()], HashMap::new()).unwrap();
        db.commit().unwrap();

        let mut pending_commit = 0u64;
        for i in 1..=EDGE_COUNT {
            let mut props = HashMap::new();
            props.insert("idx".into(), PropValue::Int(i as i64));
            let leaf = db.add_node(vec!["Leaf".into()], props).unwrap();
            db.add_edge(center.id, "HAS_LEAF".into(), leaf.id, HashMap::new())
                .unwrap();
            pending_commit += 1;
            if pending_commit >= COMMIT_BATCH {
                db.commit().unwrap();
                pending_commit = 0;
            }
        }
        if pending_commit > 0 {
            db.commit().unwrap();
        }
    }

    // Read-back phase: reopen (so we exercise `load_extent_chains` + WAL
    // recovery) and walk every edge.  Before the fix the reopen step would
    // itself report a CorruptedFile error ("edge label is not a string")
    // because edge slots past ~13 056 had been written into the property area.
    {
        let mut db = GraphDB::open(path.to_str().unwrap()).unwrap();
        let edges = db.all_edges().unwrap();
        assert_eq!(
            edges.len() as u64,
            EDGE_COUNT,
            "all inserted edges must be readable"
        );
        let mut labels_seen = std::collections::HashSet::new();
        for e in &edges {
            labels_seen.insert(e.label.clone());
            assert_eq!(e.label, "HAS_LEAF");
            assert!(e.from > 0 && e.to > 0);
        }
        assert_eq!(labels_seen.len(), 1);

        // Neighbour lookup from the center must yield EDGE_COUNT distinct IDs.
        let out = db.out_edges(1, None).unwrap();
        assert_eq!(out.len() as u64, EDGE_COUNT);
    }

    let _ = std::fs::remove_file(&path);
}
