# TSG

TSG is an embedded transactional semantic graph engine for Rust. It keeps graph
records and canonical embeddings together in `SQLite`, then derives exact and
`USearch` vector-search paths from that authoritative state.

TSG is designed for code intelligence and other local semantic systems that need
atomic graph and embedding updates without operating separate graph, relational,
and vector databases.

> **Project status:** TSG is pre-1.0 and its public API may still evolve. The
> package is publish-ready but is not currently published to crates.io. The
> repository is private, so external consumers need repository access or a
> vendored/path dependency.

## Why TSG

Keeping a graph database and a vector database synchronized creates a second
distributed-systems problem inside an application. TSG instead uses one durable
transaction boundary:

```text
application
    |
    v
TSG transaction
    |
    +-- SQLite: nodes, edges, canonical embeddings, generation
    |
    +-- exact cosine search: deterministic baseline
    |
    `-- USearch HNSW sidecar: rebuildable acceleration
```

The sidecar is never the source of truth. If it is missing, stale, or corrupt,
TSG can rebuild it from `SQLite` without regenerating embeddings.

## Capabilities

- Atomic batches containing node, edge, and embedding changes.
- Stable string node IDs backed by collision-free integer storage keys.
- Repository-local, directional, cycle-safe, bounded graph traversal.
- Exact cosine search and approximate `USearch` HNSW search.
- Adaptive backend selection based on the filtered candidate count.
- Search filters for repository and node kind.
- Transactional cascading node deletion.
- Generation tracking across canonical and accelerated state.
- Single-writer process exclusion with coexisting read-only handles.
- Full-durability defaults and an explicit lower-sync mode.
- Automatic forward schema migrations with pre-migration backups.
- Integrity reporting and explicit accelerator repair.

TSG does not generate embeddings, parse source code, expose a server, implement
full-text search, or provide an unbounded graph-query language. Those concerns
belong in the consuming application.

## Platform and compatibility

- Rust 1.83 or newer.
- macOS and Linux on local filesystems.
- One writable process per database path; read-only processes may coexist.
- Local embedded operation only. Network filesystems and distributed writers
  are outside the correctness contract.

The embedding dimension is fixed when a store is opened and must match the
dimension recorded in an existing database.

## Installation

For another crate in the same workspace or machine:

```toml
[dependencies]
tsg = { path = "../tsg" }
```

Consumers with access to the private repository can pin a Git revision:

```toml
[dependencies]
tsg = { git = "https://github.com/leonardoventurini/tsg", rev = "<commit-sha>" }
```

Pin production consumers to an immutable revision until versioned crates.io
releases begin.

## Quick start

The following example writes two nodes, their embeddings, and a relationship in
one transaction, then searches and traverses the result:

```rust,no_run
use tsg::{
    Direction, Edge, Embedding, Node, SearchBackend, SearchFilter, Store,
    WriteBatch,
};

fn main() -> tsg::Result<()> {
    const DIMENSIONS: usize = 4;

    let mut store = Store::builder("graph.db", DIMENSIONS)
        .exact_search_threshold(10_000)
        .build()?;

    let receipt = store.apply_batch(&WriteBatch {
        nodes: vec![
            Node {
                id: "parser".into(),
                repository_id: 1,
                kind: "function".into(),
                name: "parse".into(),
                content: "fn parse(source: &str) { /* ... */ }".into(),
            },
            Node {
                id: "indexer".into(),
                repository_id: 1,
                kind: "function".into(),
                name: "index".into(),
                content: "fn index() { parse(\"...\"); }".into(),
            },
        ],
        edges: vec![Edge {
            source_id: "indexer".into(),
            target_id: "parser".into(),
            relationship: "calls".into(),
        }],
        embeddings: vec![
            Embedding {
                node_id: "parser".into(),
                vector: vec![1.0, 0.0, 0.0, 0.0],
            },
            Embedding {
                node_id: "indexer".into(),
                vector: vec![0.9, 0.1, 0.0, 0.0],
            },
        ],
    })?;

    assert_eq!(receipt.generation, 1);

    let matches = store.search(
        &[1.0, 0.0, 0.0, 0.0],
        5,
        SearchFilter {
            repository_id: Some(1),
            kind: Some("function".into()),
        },
        SearchBackend::Adaptive,
    )?;

    let callees = store.traverse(
        "indexer",
        Direction::Outgoing,
        Some("calls"),
        1,
        100,
    )?;

    assert_eq!(matches.hits[0].node.id, "parser");
    assert_eq!(callees[0].id, "parser");

    Ok(())
}
```

## Data model

| Type | Meaning |
| --- | --- |
| `Node` | A stable ID, repository ID, kind, name, and source-derived content. |
| `Edge` | A typed, directed relationship between two nodes. |
| `Embedding` | One canonical `f32` vector associated with a node. |
| `WriteBatch` | Nodes, edges, and embeddings committed atomically. |
| `SearchHit` | A matching node and its cosine distance. |

Node IDs are application-owned strings. Edges must reference existing nodes or
nodes included in the same batch, and both endpoints must belong to the same
repository. Duplicate node IDs or embedding node IDs within a batch are rejected.
An upsert replaces the stored fields for the same node ID; an embedding upsert
replaces that node's prior vector. Identical edges are deduplicated.

All vectors must have the store's configured dimension and contain only finite
values. Exact cosine search also rejects a zero-magnitude query.

## Opening a store

`Store::builder` exposes all opening policy:

```rust,no_run
use tsg::{Durability, Store};

fn main() -> tsg::Result<()> {
    let store = Store::builder("graph.db", 768)
        .exact_search_threshold(25_000)
        .durability(Durability::Full)
        .read_only(false)
        .build()?;

    assert!(!store.is_read_only());
    Ok(())
}
```

| Setting | Default | Effect |
| --- | --- | --- |
| dimensions | required | Enforces one vector shape for the store. |
| exact-search threshold | `10_000` | Adaptive search uses exact search at or below this filtered candidate count. |
| durability | `Durability::Full` | Synchronizes acknowledged canonical and sidecar writes. |
| read-only | `false` | Opens a writer and acquires the per-database writer lock. |

`Store::open(path, dimensions, threshold)` is a convenience for a writable,
full-durability store with an explicit threshold.

## Transactions and generations

`apply_batch` validates the complete batch before committing it. Nodes, edges,
embeddings, and the canonical generation advance in one `SQLite` transaction.
An invalid vector, missing endpoint, cross-repository edge, or other validation
failure rolls back the whole batch.

Each successful non-empty batch advances the generation. After the canonical
commit, TSG refreshes the `USearch` sidecar. `CommitReceipt::accelerator_ready`
reports whether that refresh succeeded. A sidecar failure does not turn an
already durable canonical commit into an error; exact search remains available.

Applications should persist or publish downstream events only after receiving a
successful commit receipt.

## Deletion

`delete_nodes` removes nodes and cascades to their edges and embeddings in one
transaction:

```rust,no_run
use tsg::Store;

fn main() -> tsg::Result<()> {
    let mut store = Store::open("graph.db", 768, 10_000)?;
    let receipt = store.delete_nodes(&["obsolete-node".to_owned()])?;

    println!("deleted {} nodes", receipt.nodes_deleted);
    Ok(())
}
```

Unknown IDs are ignored. An empty request or duplicate IDs are rejected. The
generation advances only when at least one node is deleted.

## Vector search

`search(query, limit, filter, backend)` supports three policies:

| Backend | Behavior |
| --- | --- |
| `SearchBackend::Exact` | Scans canonical embeddings and returns deterministic cosine-distance results. |
| `SearchBackend::Usearch` | Uses the HNSW sidecar and fails if acceleration is unavailable. |
| `SearchBackend::Adaptive` | Uses exact search for small filtered sets or unavailable acceleration, otherwise `USearch`. |

Lower distance means greater cosine similarity. Results are ordered by distance
and then node ID for stable ties. `SearchResults::backend` reports the backend
actually used, which is useful for telemetry and performance diagnosis.

Filters can restrict `repository_id`, `kind`, both, or neither. The current
accelerated implementation searches the sidecar and post-filters candidates, so
highly selective filters do not yet provide index-level pruning.

Exact search is the correctness baseline. Approximate HNSW search can trade
recall for speed and should be evaluated with representative embeddings and
query distributions before choosing a production threshold.

## Graph traversal

`traverse` performs breadth-first traversal from a node. Callers choose
`Direction::Outgoing`, `Direction::Incoming`, or `Direction::Both`, may restrict
the relationship type, and must supply both `max_hops` and `max_results`.

Traversal never returns the starting node, is cycle-safe, stays within the
starting node's repository, and returns each reached node at most once. It is a
bounded neighborhood primitive, not a general graph-query language.

## Read-only access and concurrency

A writable open acquires `project.tsg.lock`; a second writer fails with
`Error::WriterLocked`. Read-only stores may coexist with the writer and reject
all mutation APIs.

Read-only open never creates, migrates, rebuilds, or repairs persistent state.
If its sidecar is unavailable, adaptive search falls back to exact search while
explicit `SearchBackend::Usearch` reports `Error::AcceleratorUnavailable`.

Keep each store under a clear application owner and serialize mutation through
that owner. TSG coordinates processes at the writer boundary; it is not a
distributed write coordinator.

## Durability and crash recovery

`Durability::Full` is the production default. It uses `SQLite`
`synchronous=FULL` and synchronizes generation-tagged sidecar replacement before
reporting it ready. `Durability::Normal` reduces synchronization overhead but may
lose the newest commits on power failure; process-crash atomicity is retained.

On writable open, TSG compares the canonical generation with the sidecar marker.
A missing, stale, or unreadable sidecar is rebuilt from canonical embeddings.
Abrupt writer termination releases the operating-system lock, allowing the next
writer to recover `SQLite` through its WAL and validate derived state.

Never treat the sidecar as a backup. Back up the authoritative database using
the `SQLite` backup mechanism or while the writer is stopped; copying only the
main database file can omit committed WAL content.

## Schema migrations

Writable open automatically applies supported forward migrations. Before a
migration, TSG creates a consistent backup named:

```text
project.db.schema-v<version>-<timestamp>.backup
```

`Store::migration_backup` reports the backup created by the current open. A
database with a newer unsupported schema version fails closed. Read-only opens
never migrate. Retain backups until the migrated store passes application-level
verification.

## Health, repair, and observability

- `generation` returns the authoritative generation.
- `node_count` and `embedding_count` expose basic cardinality.
- `stats` returns consolidated store statistics.
- `verify_integrity` checks `SQLite`, foreign keys, embedding byte lengths, and
  accelerator generation alignment without mutating state.
- `repair_accelerator` explicitly rebuilds the sidecar on a writable store.

Run integrity verification before repair. If canonical `SQLite` integrity fails,
restore a known-good backup; accelerator repair cannot reconstruct canonical
data. See [operations and recovery](docs/operations.md) for the deployment and
incident procedure.

## Owned files

For a database named `project.db`, TSG may own:

| Path | Purpose |
| --- | --- |
| `project.db`, `project.db-wal`, `project.db-shm` | Authoritative `SQLite` database and WAL state. |
| `project.tsg.lock` | Advisory single-writer lock. |
| `project.usearch` | Rebuildable HNSW accelerator. |
| `project.usearch.generation` | Sidecar generation marker. |
| `project.db.schema-v*-*.backup` | Pre-migration database backups. |

Treat every file as sensitive because nodes and embeddings can disclose source
content or derived information. TSG does not encrypt data at rest; use filesystem
permissions and platform encryption appropriate to the deployment.

## Errors

Public operations return `tsg::Result<T>` with typed `tsg::Error` variants for
invalid input, read-only mutation, writer contention, incompatible schemas,
unavailable acceleration, I/O, `SQLite`, and `USearch` failures. Match only the
variants for which the application has a distinct recovery policy; propagate or
record the rest with their error chain.

## Performance and scaling

The target design envelope is up to one million embeddings, but workload shape,
dimensions, filter selectivity, batch size, hardware, and recall requirements all
matter. Normal CI exercises 5,000 procedurally generated 32-dimensional vectors.
The ignored one-million-vector harness must be run on a capacity host before
making measured million-scale latency or memory claims.

Two current implementation characteristics deserve explicit planning:

- The `USearch` sidecar is rebuilt after each changed batch, so ingestion should
  use large, bounded batches.
- Accelerated filtered search currently post-filters the HNSW results rather than
  maintaining per-filter indexes.

These affect throughput and filtered recall at scale, not canonical durability.
Use exact search as the comparison oracle when tuning adaptive thresholds.

## Development and verification

The repository includes unit, property, integration, end-to-end, lifecycle,
migration, recovery, isolation, and performance tests. The standard release
checks are:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo llvm-cov --all-targets --all-features --fail-under-lines 85
cargo deny check
cargo package --locked
```

Run the capacity harness separately on a suitable host:

```sh
cargo test --test performance one_million_vector_scale_harness -- --ignored
```

Record hardware, dimensions, latency percentiles, resident memory, disk usage,
and recall against exact search with capacity results.

See the
[contribution guide](https://github.com/leonardoventurini/tsg/blob/main/CONTRIBUTING.md)
for the development workflow, the
[changelog](https://github.com/leonardoventurini/tsg/blob/main/CHANGELOG.md) for
release history, and the
[security policy](https://github.com/leonardoventurini/tsg/blob/main/SECURITY.md)
for vulnerability reporting.

## License

TSG is available under the
[MIT License](https://github.com/leonardoventurini/tsg/blob/main/LICENSE).
