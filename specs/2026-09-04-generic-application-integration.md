# Generic application integration primitives

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Problem

TSG's transactional graph and embedding core is suitable for SCS, but the 0.1
model cannot retain application-defined node and edge data, map external scopes,
store application catalog records, or perform the indexed lookups required by a
real consumer. Adding SCS-specific repositories, file hashes, or qualified names
to TSG would compromise its independent package boundary.

## Evidence and uncertainty

SCS currently needs JSON metadata, edge weights and metadata, repository path
mapping, ingestion records, qualified-name lookup, pagination, missing-embedding
queries, batch reads, and concurrent read access. Other consumers will need
equivalent application-defined data under different names.

`SQLite` supports canonical JSON validation and extraction, but arbitrary JSON
paths do not automatically receive efficient indexes. The design must therefore
make indexed attributes explicit and bounded. Representative SCS benchmarks will
determine whether the generic primitives meet its performance ceilings.

## Contracts

- TSG remains independent of SCS and contains no source-code, repository, file,
  language, qualified-name, or ingestion-specific vocabulary.
- Nodes and edges may carry validated canonical JSON attributes.
- Edges may carry a stable external ID and finite weight without changing their
  directional graph semantics.
- External scopes map stable application keys to TSG integer scope IDs.
- Namespaced catalog records store validated JSON values by stable key.
- A write transaction may atomically combine graph, embedding, and catalog
  mutations and returns one durable generation receipt.
- Attribute indexes are declared at store creation from validated JSON paths and
  are queryable through typed equality filters.
- Read APIs cover stable pagination, batch fetch, missing-vector discovery,
  counts, and bounded traversal without exposing a raw SQL connection.
- One writer and concurrent read-only handles remain the process contract.
- Version 0.1 databases are not upgraded in place for this breaking pre-1.0
  model. Consumers create a fresh 0.2 store and explicitly rebuild.

## Risks

- A broad generic API can recreate a relational database abstraction poorly.
- JSON attribute indexes can become unbounded or accept unsafe SQL fragments.
- Larger transactional batches can increase sidecar rebuild cost.
- Adding fields is a breaking Rust struct change and requires TSG 0.2.0.

Mitigations are a deliberately narrow document/catalog abstraction, strict path
validation, explicit limits, procedural tests, and exact-search correctness
oracles.

## Recovery

`SQLite` remains authoritative. Failed transactions roll back as a unit.
Sidecars rebuild from canonical embeddings. Consumers retain their previous
database until the new store completes application-level verification.

## Direct rollout

Implement tests before or with each primitive, update public documentation and
operations guidance, run the complete release matrix, publish TSG 0.2.0, and
provide its immutable tag and commit for consumers.

## Verification

- Unit/property tests for JSON validation, path validation, atomic rollback, and
  namespace isolation.
- E2E tests for catalog plus graph plus embeddings across reopen.
- Concurrent writer/read-only lifecycle tests.
- Exact/accelerated parity and sidecar recovery.
- Strict format, lint, docs, 85% coverage, supply-chain, package, macOS, Linux,
  and latest-stable Rust gates.

## Executable checklist

- [x] Design public types and test acceptance contracts.
- [x] Implement schema and transaction evolution.
- [x] Implement typed generic read/query APIs.
- [x] Add lifecycle, property, migration-rejection, and E2E coverage.
- [x] Update README, operations guide, changelog, and decision record.
- [ ] Run all local and GitHub verification.
- [ ] Publish TSG 0.2.0 and verify release assets.
