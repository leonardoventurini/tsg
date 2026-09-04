use tempfile::TempDir;
use tsg::{
    Direction, Durability, Edge, Embedding, Node, SearchBackend, SearchFilter, Store, WriteBatch,
};

const DIMENSIONS: usize = 16;

fn embedding(axis: usize) -> Vec<f32> {
    let mut vector = vec![0.0; DIMENSIONS];
    vector[axis] = 1.0;
    vector
}

#[test]
fn complete_public_api_lifecycle_survives_reopen() {
    let directory = TempDir::new().unwrap();
    let database_path = directory.path().join("project.db");
    {
        let mut store = Store::builder(&database_path, DIMENSIONS)
            .durability(Durability::Full)
            .exact_search_threshold(1)
            .build()
            .unwrap();
        let batch = WriteBatch {
            nodes: vec![
                Node {
                    id: "repository-a:file".to_string(),
                    repository_id: 10,
                    kind: "file".to_string(),
                    name: "lib.rs".to_string(),
                    content: "mod search;".to_string(),
                },
                Node {
                    id: "repository-a:function".to_string(),
                    repository_id: 10,
                    kind: "function".to_string(),
                    name: "search".to_string(),
                    content: "fn search() {}".to_string(),
                },
                Node {
                    id: "repository-b:function".to_string(),
                    repository_id: 20,
                    kind: "function".to_string(),
                    name: "unrelated".to_string(),
                    content: "fn unrelated() {}".to_string(),
                },
            ],
            edges: vec![Edge {
                source_id: "repository-a:file".to_string(),
                target_id: "repository-a:function".to_string(),
                relationship: "contains".to_string(),
            }],
            embeddings: vec![
                Embedding {
                    node_id: "repository-a:file".to_string(),
                    vector: embedding(0),
                },
                Embedding {
                    node_id: "repository-a:function".to_string(),
                    vector: embedding(1),
                },
                Embedding {
                    node_id: "repository-b:function".to_string(),
                    vector: embedding(1),
                },
            ],
        };
        let commit = store.apply_batch(&batch).unwrap();
        assert!(commit.accelerator_ready);

        let hits = store
            .search(
                &embedding(1),
                10,
                SearchFilter {
                    repository_id: Some(10),
                    kind: Some("function"),
                },
                SearchBackend::Adaptive,
            )
            .unwrap();
        assert_eq!(hits.hits.len(), 1);
        assert_eq!(hits.hits[0].node.id, "repository-a:function");
        let related = store
            .traverse(
                "repository-a:file",
                Direction::Outgoing,
                Some("contains"),
                1,
                10,
            )
            .unwrap();
        assert_eq!(related[0].id, "repository-a:function");

        store
            .delete_nodes(&["repository-a:function".to_string()])
            .unwrap();
    }

    let reader = Store::builder(&database_path, DIMENSIONS)
        .read_only(true)
        .build()
        .unwrap();
    let stats = reader.stats().unwrap();
    assert_eq!(stats.generation, 2);
    assert_eq!(stats.node_count, 2);
    assert_eq!(stats.edge_count, 0);
    assert_eq!(stats.embedding_count, 2);
    assert!(stats.read_only);
    assert!(reader.verify_integrity().unwrap().is_healthy());
}
