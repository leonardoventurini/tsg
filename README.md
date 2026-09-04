# TSG

TSG is a transactional semantic graph storage engine. `SQLite` is the durable
authority for graph records and canonical embeddings; exact and `USearch` vector
indexes are query accelerators derived from that state.

The initial crate is an internal foundation for SCS, not a general-purpose
graph database or a stable public API.

## Architecture

- `SQLite` atomically stores nodes, edges, canonical embeddings, and generations.
- Stable public node IDs map to collision-free `SQLite` integer keys.
- Exact cosine search is the deterministic baseline for small candidate sets.
- A generation-tagged `USearch` HNSW sidecar accelerates larger candidate sets.
- Missing or stale sidecars rebuild from `SQLite` without re-embedding.
- Graph traversal is directional, cycle-safe, repository-local, and bounded.

The caller supplies the adaptive exact-search threshold because representative
SCS benchmarks have not yet established a production default.

## Operational guarantees

- Full durability is the default. A commit synchronizes `SQLite` through
  `synchronous=FULL` and synchronizes generation-tagged sidecar replacements.
- One writable process owns a database path. Read-only handles may coexist and
  never migrate, repair, or create files.
- Automatic forward migrations create a consistent backup before changing the
  schema. Databases from newer schema versions fail closed.
- `SQLite` is authoritative. Missing, stale, or corrupt `USearch` files rebuild
  without calling an embedding provider.
- A committed write remains successful when sidecar refresh fails; the receipt
  reports degraded acceleration and exact search remains available.

TSG supports local filesystems on macOS and Linux. Network filesystems,
distributed writers, embedding generation, and unbounded graph queries are not
part of its correctness contract. See [operations](docs/operations.md) for
recovery and deployment guidance.

## Minimal usage

```rust,no_run
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
cargo llvm-cov --all-targets --all-features --fail-under-lines 85
```

The one-million-vector design-envelope harness is intentionally excluded from
normal CI. Run it on a capacity-test host with:

```sh
cargo test --test performance one_million_vector_scale_harness -- --ignored
```
