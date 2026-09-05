# Changelog

All notable changes are recorded here. TSG follows Semantic Versioning.

## [Unreleased]

## [0.2.3] - 2026-09-05

### Fixed

- Embedding upserts update only changed USearch keys instead of reconstructing
  the entire vector index after every batch. Metadata-only writes reuse the
  accelerator. Failed post-commit updates discard partial accelerator state;
  canonical exact search and reopen/repair preserve availability and recovery.

## [0.2.2] - 2026-09-05

### Fixed

- Scoped attribute lookups use registered SQLite expression indexes instead of
  scanning every node for each lookup. JSON path validation, value binding,
  pagination, and unscoped query behavior remain unchanged.

## [0.2.1] - 2026-09-04

### Fixed

- Writable adaptive search falls back to canonical exact search when sidecar
  persistence fails, matching read-only availability behavior.
- Node updates cannot leave existing edges crossing application scopes; invalid
  batches roll back atomically.
- Name searches treat backslashes literally alongside percent and underscore.

## [0.2.0] - 2026-09-04

### Added

- Generic application scopes, node and edge JSON attributes, stable weighted
  edge identities, and opt-in indexed attribute paths.
- Transactional namespaced catalog records committed with graph and embedding
  mutations.
- Stable paginated node reads, attribute equality queries, missing-vector
  discovery, filtered counts, name and batch reads, edge reads, and edge
  deletion.
- Explicit embedding reset, full-store truncation, and vacuum operations.

### Changed

- Replaced repository-specific terminology with generic application scopes.
- Version 0.1 and legacy stores now fail with `Error::ReindexRequired`; consumers
  must build a fresh 0.2 store.

## [0.1.1] - 2026-09-04

### Changed

- Adopted the Rust 2024 edition and raised the minimum supported Rust version
  from 1.83 to 1.98 as a latest-stable toolchain policy.
- Updated all direct dependencies and GitHub Actions to their latest available
  releases.
- Made dependency-resolving CI checks reproducible with the committed lockfile.

## [0.1.0] - 2026-09-04

### Added

- SQLite-authoritative nodes, edges, canonical embeddings, and generations.
- Exact and USearch cosine retrieval with adaptive backend selection.
- Transactional upsert and cascading node deletion.
- Bounded repository-local traversal.
- Full-durability default and optional normal-durability mode.
- Cross-process writer exclusion and non-mutating read-only handles.
- Backed automatic schema migration and future-version rejection.
- Integrity diagnostics, explicit accelerator repair, statistics, and recovery.
- Unit, integration, E2E, property, migration, concurrency, recovery, and
  performance validation.
