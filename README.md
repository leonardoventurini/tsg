# TSG

TSG is a transactional semantic graph storage engine. SQLite is the durable
authority for graph records and canonical embeddings; exact and USearch vector
indexes are query accelerators derived from that state.

The initial crate is an internal foundation for SCS, not a general-purpose
graph database or a stable public API.

## Architecture

- SQLite atomically stores nodes, edges, canonical embeddings, and generations.
- Stable public node IDs map to collision-free SQLite integer keys.
- Exact cosine search is the deterministic baseline for small candidate sets.
- A generation-tagged USearch HNSW sidecar accelerates larger candidate sets.
- Missing or stale sidecars rebuild from SQLite without re-embedding.
- Graph traversal is directional, cycle-safe, repository-local, and bounded.

The caller supplies the adaptive exact-search threshold because representative
SCS benchmarks have not yet established a production default.

## Minimal usage

```rust
use tsg::{Embedding, Node, SearchBackend, SearchFilter, Store, WriteBatch};

let mut store = Store::open("graph.db", 1_536, 10_000)?;
store.apply_batch(&WriteBatch {
    nodes: vec![Node {
        id: "node-id".into(),
        repository_id: 1,
        kind: "function".into(),
        name: "example".into(),
        content: "fn example() {}".into(),
    }],
    embeddings: vec![Embedding {
        node_id: "node-id".into(),
        vector: vec![0.0; 1_536],
    }],
    ..WriteBatch::default()
})?;

let results = store.search(
    &vec![0.1; 1_536],
    10,
    SearchFilter::default(),
    SearchBackend::Adaptive,
)?;
# Ok::<(), tsg::Error>(())
```

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```
