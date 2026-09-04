# Production readiness

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Problem

The vertical slice proves atomic graph/embedding writes, bounded traversal, and
exact/USearch retrieval, but it lacks the operational contracts, lifecycle
operations, compatibility machinery, observability, and adversarial test depth
required for production use by SCS and independent Rust consumers.

## Evidence

The repository currently has seven in-crate functional tests. It has no schema
migration framework, backup policy, cross-process writer exclusion, deletion
API, integrity checker, metrics, integration/E2E suite, property tests,
performance gates, coverage enforcement, or release CI. The current USearch
accelerator is rebuilt after every batch, and filtered searches scan every
accelerator result to preserve correctness.

## Desired outcome and scope

Produce a publish-ready, documented Rust library for macOS and Linux that can
serve projects containing up to one million nodes and embeddings. SQLite remains
the sole durable authority; exact and USearch indexes remain derived retrieval
implementations. SCS itself is not modified.

This work includes:

- a typed builder and durability policy, defaulting to fully synchronized writes;
- exclusive cross-process writer ownership and read-only handles;
- schema versioning, automatic forward migration, and pre-migration backups;
- transactional upsert and delete operations with explicit commit receipts;
- integrity verification and deterministic accelerator repair;
- public operational statistics and backend-selection evidence;
- unit, integration, E2E, property, migration, corruption, concurrency, and
  performance coverage;
- coverage enforcement, macOS/Linux CI, security policy, changelog, license,
  package metadata, and release documentation.

Distributed operation, Windows support, network service APIs, embedding model
execution, SCS integration, and crates.io publication are outside scope.

## Assumptions and constraints

- A writable store has exactly one process owner; read-only snapshots may coexist.
- Local filesystems only; network filesystem locking is unsupported.
- Public node IDs are stable strings and internal keys are SQLite-assigned integers.
- Canonical vectors are little-endian finite `f32` values of one configured dimension.
- Repository boundaries are enforced for edges and traversals.
- `Durability::Full` configures SQLite and derived-sidecar synchronization for
  acknowledged writes; a documented `Durability::Normal` mode may trade the
  last power-loss window for throughput.
- A committed SQLite generation is successful even if accelerator refresh fails.
  Exact retrieval remains available and explicit repair can restore acceleration.
- API compatibility follows semantic versioning; the crate remains `0.x` until
  an SCS adapter validates the contract.

## Public contracts

- `StoreBuilder` is the supported open/configuration surface.
- Writable opens acquire a non-blocking exclusive lock; contention is typed.
- Read-only opens never mutate schema, backups, sidecars, or database files.
- Every schema migration first creates a SQLite-consistent backup and then runs
  transactionally. Newer unknown schemas fail closed.
- Batch validation completes before mutation where practical; any SQL failure
  rolls back the entire graph/embedding/generation change.
- Deletes cascade from nodes to edges and embeddings in the same transaction.
- Search validates dimensions, finiteness, and non-zero norm.
- Exact and accelerated results use ascending cosine distance and stable ID
  tie-breaking. Approximate recall is measured rather than assumed.
- Integrity checks never mutate state. Repair is explicit except for safe
  accelerator reconstruction during writable open or accelerated search.
- All unbounded caller inputs have configurable or explicit limits.

## Risks and mitigations

- Cross-file atomicity: SQLite is authoritative; sidecar generation mismatch
  always triggers rebuild rather than trust.
- Power failure: use SQLite `synchronous=FULL`, WAL checkpoints where required,
  file `sync_all`, atomic rename, and parent-directory synchronization.
- Migration corruption: create and validate a SQLite backup before migration;
  preserve the backup on failure.
- Lock portability: use OS advisory locks through a maintained Rust crate and
  document local-filesystem scope.
- HNSW recall drift: property fixtures compare against exact search and a
  benchmark gate records recall/latency separately.
- One-million-record memory growth: stream SQLite rows where possible and add an
  ignored scale harness plus bounded CI performance fixtures.
- API premature stabilization: publish-ready metadata is delivered, but actual
  publication and `1.0` remain separate approval gates.

## Recovery and rollback

- A missing, corrupt, or stale USearch sidecar is deleted and rebuilt from
  canonical SQLite embeddings.
- Failed migrations restore by replacing the database with the preserved backup.
- SQLite integrity failure is reported without attempting speculative repair.
- Every implementation unit is committed separately and can be reverted.
- No SCS state or source changes occur, so rollback cannot affect SCS.

## Direct rollout

There is no external rollout. CI verifies macOS and Linux using the minimum
supported and stable Rust toolchains. Release packaging is dry-run only. A later
SCS integration must begin with an inactive adapter and shadow-read validation.

## Verification matrix

- Formatting: `cargo fmt --check`.
- Static analysis: strict Clippy with all targets/features and warnings denied.
- Unit/integration/E2E: `cargo test --all-targets --all-features`.
- Documentation: `cargo test --doc` and `cargo doc --no-deps` with warnings denied.
- Coverage: `cargo llvm-cov` with a documented minimum line threshold.
- Supply chain: `cargo deny check` in CI.
- Performance: deterministic CI budgets plus an ignored one-million-record harness.
- Packaging: `cargo package --allow-dirty` without publishing.

## Executable checklist

- [x] Agree production, compatibility, durability, platform, and distribution scope.
- [ ] Introduce builder, durability, writer locking, read-only behavior, and typed errors.
- [ ] Add schema migrations with pre-migration backup and future-version rejection.
- [ ] Add transactional deletion, integrity reporting, repair, and statistics.
- [ ] Harden sidecar persistence, corruption recovery, and generation semantics.
- [ ] Add integration, E2E, property, migration, corruption, and concurrency tests.
- [ ] Add deterministic performance/recall gates and a one-million-record harness.
- [ ] Add coverage, supply-chain, macOS/Linux, MSRV, documentation, and package CI.
- [ ] Add public documentation, security policy, changelog, license, and release guidance.
- [ ] Run the complete local verification matrix that is available in the environment.
- [ ] Record production architecture and compatibility decisions.
- [ ] Commit every verified unit with path-limited staging.

