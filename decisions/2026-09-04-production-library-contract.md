# Establish the production library contract

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Context

The initial vertical slice proved the storage model but did not define stable
lifecycle, durability, migration, concurrency, recovery, or distribution
behavior. TSG is intended for both eventual SCS consumption and independent Rust
consumers on macOS and Linux.

## Decision

TSG remains pre-1.0 but exposes a documented publish-ready Rust API through
`StoreBuilder`. Full power-loss durability is the default. Writable opens take a
non-blocking process-exclusive advisory lock; read-only handles coexist without
mutating any artifact.

Schema versions advance through automatic transactional migrations. Every
migration of an existing store first creates a SQLite-consistent, synchronized
backup. Unknown future schemas fail closed. SQLite remains authoritative, while
USearch corruption or generation drift is repaired from canonical vectors.

Production gates include strict formatting/lints/docs, public E2E and property
tests, cross-process locking, migration and recovery coverage, an 85% line
coverage floor, supply-chain policy, deterministic CI performance budgets, an
opt-in one-million-record harness, package verification, and macOS/Linux CI.
The toolchain policy subsequently changed to the latest stable Rust release; see
`2026-09-04-latest-stable-rust.md`.

## Rejected alternatives

- Default to relaxed durability: surprising data-loss windows are inappropriate
  for a storage engine.
- Permit multiple writers and reconcile later: conflicts with the single SQLite
  authority and complicates sidecar generation ownership.
- Automatically repair SQLite corruption: risks compounding data loss; only
  derived sidecars are safe to reconstruct.
- Claim one-million-vector performance from small CI fixtures: the capacity
  envelope requires recorded execution on representative hardware.
- Publish immediately: package readiness does not replace downstream adapter and
  shadow-read validation.

## Consequences

- Default commits incur synchronization cost.
- Local-filesystem advisory locking is part of the correctness model.
- Migration backups consume disk until operators retire them.
- The current full USearch rebuild remains the primary ingestion-scaling
  limitation and is explicitly documented.
- SCS remains unchanged; its eventual adoption is a separate reviewed project.
