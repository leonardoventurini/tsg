# Incremental vector accelerator upserts

## Problem and evidence

SCS ingestion with 4,096-dimensional embeddings spends most wall time rebuilding
TSG's full USearch index after each 32-vector write. Runtime sampling attributes
CPU saturation to `VectorAccelerator::rebuild`; eight model requests produced 256
vectors in 34.67 seconds across a 676.228-second ingestion interval. `apply_batch`
currently reconstructs every stored vector, even for metadata-only writes.

## Contract and design

SQLite remains authoritative. Capture its generation inside the transaction before
mutation. After commit, update only the batch's embedding keys when the accelerator
matches that prior generation; replacements remove the old key before adding its
new vector. Persist through the existing atomic sidecar/generation protocol before
marking the accelerator current. Missing/stale accelerators rebuild from SQLite.
Any incremental update or persistence failure discards the potentially partial
accelerator and returns a committed receipt with `accelerator_ready = false`.
Adaptive exact search and explicit repair/reopen remain available. Public APIs,
SQLite format, sidecar format, dependencies and security boundaries do not change.
Metadata-only batches still persist the sidecar but do not reconstruct ANN links.

## Risks and recovery

USearch replacement is remove/add, so failure between those steps must never leave
a searchable partial accelerator. The mutex protects readers during the update.
Disk persistence is still proportional to corpus size; this change removes repeated
ANN construction, not all write amplification. Existing rebuild remains the recovery
path. Rollback means consuming the previous immutable release; SQLite is compatible.

## Executable verification and rollout

- [x] Generated multi-batch identity regression fails before implementation.
- [x] Generated additions/replacements preserve search correctness and reopen state.
- [x] Failed SQL leaves index untouched; failed sidecar persistence degrades safely.
- [x] Missing/stale accelerator recovers from authoritative SQLite.
- [x] Run a bounded generated 4,096-dimensional, 32-vector batch benchmark before/after.
- [x] Run affected tests and required formatting, clippy, tests, docs, package,
      coverage and dependency-security gates; record actual results.
- [x] Commit task-owned changes with hooks enabled. Coordinate immutable release
      and SCS dependency update with parent; do not restart active ingestion.

## Executed verification

The new accelerator-identity regression failed before the implementation at the
first embedding batch (instance 1 versus initial instance 0). After the fix,
43 tests passed with two opt-in diagnostics ignored; line coverage is 89.26%.
Formatting, strict all-target clippy, warning-free rustdoc, package build/verification,
and cargo-deny passed. Cargo-deny retains existing duplicate-transitive-version
and unused-license-allowance warnings; advisories, bans, licenses and sources pass.
The ignored million-vector diagnostic was not run. The new moderate diagnostic
was run explicitly both before and after the production change:

`cargo test --locked --test incremental_vectors generated_4096_dimension_batch_throughput -- --ignored --nocapture`

On the same local debug build, 512 generated dense vectors with 4,096 dimensions
and sixteen 32-vector commits took 4.589453833 seconds before and 1.038200792 seconds
after (~4.4x faster). This is a bounded synthetic observation, not a production
latency guarantee or a CI timing threshold. Tests also check replacement distance,
key uniqueness, current node metadata, SQL rollback, sidecar failure fallback,
stale/missing accelerator recovery, and successful read-only persisted reopen.

Release metadata and README examples now target immutable v0.2.3. Publication is
pending remote CI; the active SCS daemon was neither restarted nor mutated.
