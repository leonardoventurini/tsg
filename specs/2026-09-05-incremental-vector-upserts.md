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

Published immutable v0.2.3 at `f3f45241723a83b322676ecd22c36d4add66a53b` after
CI `33990040206` passed all jobs. Release workflow `33990185824` passed and
published `tsg-0.2.3.crate` plus `SHA256SUMS`. Downloaded archive verification passed;
SHA-256: `309c52d3349260150ad0fbfbce4a8b873d04ba0b815845a18ab53d0a8b28ee0e`.
Release: https://github.com/leonardoventurini/tsg/releases/tag/v0.2.3
The active SCS daemon was neither restarted nor mutated by this upstream work.
Consumer SCS v0.1.8 pins the tag and passed 254 Python tests, 99 Rust tests and
84.67% Python line coverage before its separate release CI.
