use tempfile::TempDir;
use tsg::{
    Direction, Edge, Embedding, Error, Node, SearchBackend, SearchFilter, Store, WriteBatch,
};

const DIMENSIONS: usize = 8;

fn vector(axis: usize) -> Vec<f32> {
    let mut vector = vec![0.0; DIMENSIONS];
    vector[axis] = 1.0;
    vector
}

fn batch() -> WriteBatch {
    WriteBatch {
        nodes: (0..3)
            .map(|index| Node {
                id: format!("node-{index}"),
                repository_id: 1,
                kind: "function".to_string(),
                name: format!("function_{index}"),
                content: String::new(),
            })
            .collect(),
        edges: vec![
            Edge {
                source_id: "node-0".to_string(),
                target_id: "node-1".to_string(),
                relationship: "calls".to_string(),
            },
            Edge {
                source_id: "node-1".to_string(),
                target_id: "node-2".to_string(),
                relationship: "calls".to_string(),
            },
        ],
        embeddings: (0..3)
            .map(|index| Embedding {
                node_id: format!("node-{index}"),
                vector: vector(index),
            })
            .collect(),
    }
}

#[test]
fn delete_cascades_and_remains_correct_after_reopen() {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("graph.db");
    {
        let mut store = Store::open(&database_path, DIMENSIONS, 1).unwrap();
        store.apply_batch(&batch()).unwrap();

        let receipt = store.delete_nodes(&["node-1".to_string()]).unwrap();

        assert_eq!(receipt.generation, 2);
        assert_eq!(receipt.nodes_deleted, 1);
        assert!(receipt.accelerator_ready);
        assert_eq!(store.stats().unwrap().edge_count, 0);
        assert_eq!(store.embedding_count().unwrap(), 2);
        assert!(store
            .traverse("node-0", Direction::Outgoing, None, 2, 10)
            .unwrap()
            .is_empty());
    }

    let reopened = Store::open(&database_path, DIMENSIONS, 1).unwrap();
    let hits = reopened
        .search(&vector(1), 3, SearchFilter::default(), SearchBackend::Exact)
        .unwrap();
    assert!(!hits.hits.iter().any(|hit| hit.node.id == "node-1"));
    assert!(reopened.verify_integrity().unwrap().is_healthy());
}

#[test]
fn corrupt_sidecar_is_rebuilt_from_canonical_vectors() {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("graph.db");
    {
        let mut store = Store::open(&database_path, DIMENSIONS, 1).unwrap();
        store.apply_batch(&batch()).unwrap();
    }
    std::fs::write(database_path.with_extension("usearch"), b"corrupt").unwrap();

    let reopened = Store::open(&database_path, DIMENSIONS, 1).unwrap();

    assert!(reopened.verify_integrity().unwrap().is_healthy());
    let hits = reopened
        .search(
            &vector(2),
            1,
            SearchFilter::default(),
            SearchBackend::Usearch,
        )
        .unwrap();
    assert_eq!(hits.hits[0].node.id, "node-2");
}

#[test]
fn readonly_adaptive_search_falls_back_when_sidecar_is_missing() {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("graph.db");
    {
        let mut store = Store::open(&database_path, DIMENSIONS, 0).unwrap();
        store.apply_batch(&batch()).unwrap();
    }
    std::fs::remove_file(database_path.with_extension("usearch")).unwrap();
    let reader = Store::builder(&database_path, DIMENSIONS)
        .exact_search_threshold(0)
        .read_only(true)
        .build()
        .unwrap();

    let adaptive = reader
        .search(
            &vector(0),
            1,
            SearchFilter::default(),
            SearchBackend::Adaptive,
        )
        .unwrap();
    let accelerated = reader.search(
        &vector(0),
        1,
        SearchFilter::default(),
        SearchBackend::Usearch,
    );

    assert_eq!(adaptive.backend, SearchBackend::Exact);
    assert_eq!(adaptive.hits[0].node.id, "node-0");
    assert!(matches!(accelerated, Err(Error::AcceleratorUnavailable(_))));
}
