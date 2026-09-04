# Changelog

All notable changes are recorded here. TSG follows Semantic Versioning.

## [Unreleased]

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
