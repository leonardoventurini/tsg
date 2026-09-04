# Transactional semantic graph vertical slice

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Problem

SCS currently commits graph state to SQLite and embedding membership to a
separate USearch sidecar. Recovery therefore depends on ordering, flushing,
reopening, and cross-store verification. TSG needs one durable authority while
retaining specialized vector retrieval.

## Evidence

The current SCS store exposes node, edge, ingestion, traversal, and vector
operations from one crate, but canonical vector payloads are not stored in the
same SQLite transaction as their nodes. USearch persistence is consequently a
separate durability boundary.

## Outcome and scope

Deliver a standalone Rust crate that provides:

- transactional batch upsert of nodes, edges, and canonical `f32` embeddings;
- collision-free integer storage keys behind stable string identifiers;
- bounded directional graph traversal;
- deterministic exact cosine search;
- USearch cosine search as a rebuildable accelerator;
- adaptive selection between exact and USearch retrieval;
- persisted generation tracking and automatic accelerator rebuilding.

SCS integration, ingestion jobs, lexical search, schema migration from SCS,
packaging, and publication are outside this vertical slice.

## Uncertainty

No SCS-specific latency or recall benchmark yet establishes the ideal adaptive
threshold. The initial threshold is caller-configurable and correctness tests
compare USearch results with the exact implementation on deterministic data.

## Contracts

- SQLite is authoritative; accelerator failure must not invalidate committed
  graph or embedding state.
- Every embedding has exactly the configured dimension and only finite values.
- Node, edge, embedding, and generation changes in a batch commit atomically.
- Edges may only reference nodes visible in the committing transaction.
- Search returns ascending cosine distance with deterministic ID tie-breaking.
- Traversal is repository-scoped, cycle-safe, and bounded by hops and results.
- A USearch sidecar is usable only when its generation equals SQLite's.
- Exact search remains available whenever canonical embeddings are readable.

## Test strategy and acceptance criteria

- Procedurally generate vector fixtures; do not store opaque fixture files.
- Prove a valid batch commits nodes, edges, and embeddings together.
- Prove invalid embedding dimensions roll the complete batch back.
- Prove bounded outgoing and incoming traversal handles cycles.
- Prove exact and USearch top-k parity for separated deterministic vectors.
- Prove reopen reconstructs a missing/stale USearch sidecar from SQLite.
- Prove adaptive search uses exact below its threshold and USearch above it.
- Run formatting, Clippy with warnings denied, and the complete test suite.

## Risks and mitigations

- Quantitative USearch recall can differ from exact search: retain exact search
  as the oracle and use unambiguous fixtures in the first contract tests.
- SQLite commit can succeed while accelerator refresh fails: report degraded
  accelerator readiness and continue serving exact search.
- Sidecar replacement can be interrupted: build into a temporary sibling,
  sync, atomically rename, and rebuild when generation evidence disagrees.
- Schema/API choices may constrain SCS later: keep the initial surface small
  and explicitly unstable until an SCS adapter validates it.

## Recovery and rollback

Delete any derived USearch sidecar and reopen TSG to rebuild it from SQLite.
Rollback of this standalone work is deletion or Git revert of the new
repository; SCS is not modified. Canonical SQLite data never depends on the
sidecar for recovery.

## Direct rollout

There is no production rollout in this unit. Build and exercise the crate as a
standalone library. A later reviewed change may add an inactive SCS adapter,
followed by measured shadow reads before any storage cutover.

## Executable checklist

- [x] Initialize the standalone repository and record the agreed architecture.
- [x] Write transactional, traversal, parity, adaptive, and reopen tests.
- [x] Implement schema, typed contracts, and transactional batch writes.
- [x] Implement bounded traversal and exact cosine retrieval.
- [x] Implement the generation-tagged USearch accelerator and adaptive planner.
- [x] Run `cargo fmt --check`.
- [x] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] Run `cargo test`.
- [x] Record the resulting architectural decision.

