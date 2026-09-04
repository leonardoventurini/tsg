# Use SQLite authority with adaptive vector indexes

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Context

TSG must keep graph structure and canonical embeddings consistent without
giving up specialized vector retrieval. A separate authoritative vector store
would preserve the cross-store commit and recovery boundary that motivated the
project. Building a pager, WAL, and transactional index format would duplicate
mature database infrastructure.

## Decision

SQLite is the sole durable authority for nodes, edges, canonical `f32`
embeddings, and the committed generation. Stable string node identifiers map
to SQLite-assigned integer keys used by edge joins and USearch.

Vector retrieval has two implementations behind the TSG search contract:

- an exact cosine scan over canonical SQLite vectors, serving as the
  deterministic correctness baseline and the path for small candidate sets;
- a generation-tagged USearch HNSW sidecar, serving larger candidate sets and
  rebuilt automatically from SQLite when absent or stale.

The adaptive threshold is configured by the caller until representative SCS
benchmarks establish a default. Filtered USearch queries exhaustively
post-filter the current accelerator in this first slice, preserving correctness
at the expense of performance until allowlist pushdown is implemented.

## Rejected alternatives

- Make USearch authoritative: retains split durability and prevents recovery
  without re-embedding.
- Implement only an exact scan: provides a good oracle but does not validate
  the intended large-corpus acceleration boundary.
- Store a custom vector index inside SQLite pages: creates substantial pager,
  migration, rollback, and corruption-recovery work before measurements justify
  it.
- Adopt LodeDB as the authority: introduces different quantization, ANN,
  packaging, and storage contracts while preserving a second database boundary.

## Rationale

One SQLite transaction can commit graph and embedding truth. Search indexes can
then fail, be deleted, change implementation, or lag a generation without data
loss. Exact retrieval supplies an independent oracle for accelerator parity,
and USearch preserves a proven HNSW option for interactive large-corpus search.

## Consequences

- Canonical embeddings consume additional SQLite space.
- The initial implementation rebuilds the complete USearch index after each
  committed batch; incremental generation replay is future work.
- Accelerator failure degrades availability to exact search rather than
  invalidating an already committed batch.
- Cross-repository edges are rejected, matching project isolation.
- The API is intentionally pre-release and will require an adapter and shadow
  verification before SCS consumes it.
