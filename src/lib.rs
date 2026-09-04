mod error;
mod store;
mod types;
mod vector;

pub use error::{Error, Result};
pub use store::{Store, StoreBuilder};
pub use types::{
    CommitReceipt, Direction, Durability, Edge, Embedding, Node, SearchBackend, SearchFilter,
    SearchHit, SearchResults, StoreStats, WriteBatch,
};

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const DIMENSIONS: usize = 8;

    fn node(id: &str, repository_id: i64, kind: &str) -> Node {
        Node {
            id: id.to_string(),
            repository_id,
            kind: kind.to_string(),
            name: format!("name_{id}"),
            content: format!("content for {id}"),
        }
    }

    fn unit_vector(axis: usize) -> Vec<f32> {
        let mut vector = vec![0.0; DIMENSIONS];
        vector[axis] = 1.0;
        vector
    }

    fn store(directory: &TempDir, threshold: usize) -> Store {
        Store::open(directory.path().join("graph.db"), DIMENSIONS, threshold).unwrap()
    }

    fn seeded_batch(count: usize) -> WriteBatch {
        let nodes = (0..count)
            .map(|index| node(&format!("node-{index}"), 7, "function"))
            .collect();
        let embeddings = (0..count)
            .map(|index| Embedding {
                node_id: format!("node-{index}"),
                vector: unit_vector(index % DIMENSIONS),
            })
            .collect();
        WriteBatch {
            nodes,
            embeddings,
            ..WriteBatch::default()
        }
    }

    #[test]
    fn commits_graph_and_embeddings_in_one_generation() {
        let directory = TempDir::new().unwrap();
        let mut store = store(&directory, 4);
        let mut batch = seeded_batch(2);
        batch.edges.push(Edge {
            source_id: "node-0".to_string(),
            target_id: "node-1".to_string(),
            relationship: "calls".to_string(),
        });

        let receipt = store.apply_batch(&batch).unwrap();

        assert_eq!(receipt.generation, 1);
        assert!(receipt.accelerator_ready);
        assert_eq!(store.node_count().unwrap(), 2);
        assert_eq!(store.embedding_count().unwrap(), 2);
        let related = store
            .traverse("node-0", Direction::Outgoing, Some("calls"), 1, 10)
            .unwrap();
        assert_eq!(related[0].id, "node-1");
    }

    #[test]
    fn invalid_embedding_rolls_back_the_complete_batch() {
        let directory = TempDir::new().unwrap();
        let mut store = store(&directory, 4);
        let batch = WriteBatch {
            nodes: vec![node("invalid", 1, "function")],
            embeddings: vec![Embedding {
                node_id: "invalid".to_string(),
                vector: vec![1.0; DIMENSIONS - 1],
            }],
            ..WriteBatch::default()
        };

        assert!(matches!(
            store.apply_batch(&batch),
            Err(Error::InvalidInput(_))
        ));
        assert_eq!(store.node_count().unwrap(), 0);
        assert_eq!(store.embedding_count().unwrap(), 0);
        assert_eq!(store.generation().unwrap(), 0);
    }

    #[test]
    fn cross_repository_edge_rolls_back_the_complete_batch() {
        let directory = TempDir::new().unwrap();
        let mut store = store(&directory, 4);
        let batch = WriteBatch {
            nodes: vec![node("left", 1, "function"), node("right", 2, "function")],
            edges: vec![Edge {
                source_id: "left".to_string(),
                target_id: "right".to_string(),
                relationship: "calls".to_string(),
            }],
            ..WriteBatch::default()
        };

        assert!(matches!(
            store.apply_batch(&batch),
            Err(Error::InvalidInput(_))
        ));
        assert_eq!(store.node_count().unwrap(), 0);
        assert_eq!(store.generation().unwrap(), 0);
    }

    #[test]
    fn traversal_is_directional_bounded_and_cycle_safe() {
        let directory = TempDir::new().unwrap();
        let mut store = store(&directory, 4);
        let mut batch = seeded_batch(3);
        batch.edges = vec![
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
            Edge {
                source_id: "node-2".to_string(),
                target_id: "node-0".to_string(),
                relationship: "calls".to_string(),
            },
        ];
        store.apply_batch(&batch).unwrap();

        let one_hop = store
            .traverse("node-0", Direction::Outgoing, Some("calls"), 1, 10)
            .unwrap();
        let incoming = store
            .traverse("node-0", Direction::Incoming, Some("calls"), 2, 10)
            .unwrap();

        assert_eq!(
            one_hop.iter().map(|node| &node.id).collect::<Vec<_>>(),
            ["node-1"]
        );
        assert_eq!(incoming.len(), 2);
        assert!(!incoming.iter().any(|node| node.id == "node-0"));
    }

    #[test]
    fn exact_and_usearch_agree_for_separated_vectors() {
        let directory = TempDir::new().unwrap();
        let mut store = store(&directory, 4);
        store.apply_batch(&seeded_batch(DIMENSIONS)).unwrap();
        let query = unit_vector(3);

        let exact = store
            .search(&query, 1, SearchFilter::default(), SearchBackend::Exact)
            .unwrap();
        let accelerated = store
            .search(&query, 1, SearchFilter::default(), SearchBackend::Usearch)
            .unwrap();

        assert_eq!(exact.hits[0].node.id, "node-3");
        assert_eq!(accelerated.hits[0].node.id, exact.hits[0].node.id);
        assert!((accelerated.hits[0].distance - exact.hits[0].distance).abs() < 1e-6);
    }

    #[test]
    fn adaptive_search_uses_candidate_count_threshold() {
        let directory = TempDir::new().unwrap();
        let mut store = store(&directory, 2);
        store.apply_batch(&seeded_batch(3)).unwrap();

        let broad = store
            .search(
                &unit_vector(0),
                1,
                SearchFilter::default(),
                SearchBackend::Adaptive,
            )
            .unwrap();
        let narrow = store
            .search(
                &unit_vector(0),
                1,
                SearchFilter {
                    repository_id: Some(999),
                    kind: None,
                },
                SearchBackend::Adaptive,
            )
            .unwrap();

        assert_eq!(broad.backend, SearchBackend::Usearch);
        assert_eq!(narrow.backend, SearchBackend::Exact);
    }

    #[test]
    fn reopen_rebuilds_a_missing_accelerator_from_sqlite() {
        let directory = TempDir::new().unwrap();
        let database_path = directory.path().join("graph.db");
        {
            let mut store = Store::open(&database_path, DIMENSIONS, 4).unwrap();
            store.apply_batch(&seeded_batch(3)).unwrap();
        }
        std::fs::remove_file(database_path.with_extension("usearch")).unwrap();

        let reopened = Store::open(&database_path, DIMENSIONS, 4).unwrap();
        let results = reopened
            .search(
                &unit_vector(1),
                1,
                SearchFilter::default(),
                SearchBackend::Usearch,
            )
            .unwrap();

        assert_eq!(results.hits[0].node.id, "node-1");
        assert!(database_path.with_extension("usearch").exists());
    }

    #[test]
    fn writable_open_excludes_a_second_writer() {
        let directory = TempDir::new().unwrap();
        let database_path = directory.path().join("graph.db");
        let _owner = Store::open(&database_path, DIMENSIONS, 4).unwrap();

        let Err(error) = Store::open(&database_path, DIMENSIONS, 4) else {
            panic!("second writer unexpectedly acquired the store");
        };

        assert!(matches!(error, Error::WriterLocked(_)));
    }

    #[test]
    fn read_only_handle_coexists_and_rejects_mutation() {
        let directory = TempDir::new().unwrap();
        let database_path = directory.path().join("graph.db");
        let mut writer = Store::open(&database_path, DIMENSIONS, 4).unwrap();
        writer.apply_batch(&seeded_batch(2)).unwrap();
        let mut reader = Store::builder(&database_path, DIMENSIONS)
            .read_only(true)
            .build()
            .unwrap();

        assert!(reader.is_read_only());
        assert_eq!(reader.node_count().unwrap(), 2);
        assert!(matches!(
            reader.apply_batch(&seeded_batch(1)),
            Err(Error::ReadOnly)
        ));
    }
}
