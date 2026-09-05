# Operations and recovery

## Files

For `project.db`, TSG may own:

- `project.db`, `project.db-wal`, and `project.db-shm`: authoritative SQLite state;
- `project.tsg.lock`: advisory single-writer lock;
- `project.usearch`: rebuildable vector accelerator;
- `project.usearch.generation`: accelerator generation marker;

Protect every file as sensitive source-derived data. Do not copy a live SQLite
database and omit its WAL; use SQLite's backup mechanism or stop the writer.

## Recovery

Run `Store::verify_integrity` first. If SQLite is healthy but acceleration is
not, open writable and call `Store::repair_accelerator`. Deleting only the
`.usearch` file and its generation marker is also recoverable because a writable
open reconstructs them from canonical embeddings.

Adaptive search uses exact canonical vectors whenever acceleration is unavailable,
including after sidecar write failures on writable handles. Explicit USearch
requests still report a failure; `repair_accelerator` restores acceleration once
the filesystem problem is resolved.

Schema versions that require a rebuild fail with `Error::ReindexRequired` and
leave the existing database unchanged. Create a fresh database at a separate
path, rebuild it from the application's source of truth, verify it, and only
then retire the prior store.

An SQLite integrity failure is not repaired automatically. Restore a known-good
backup and rebuild the sidecar. Never treat the sidecar as a backup of canonical
embeddings.

## Durability

`Durability::Full` is the default and is intended for acknowledged production
writes. `Durability::Normal` reduces synchronization overhead but may lose the
latest committed changes on power failure even though process-crash atomicity is
retained. Select it only through an explicit application policy.

## Capacity

Normal CI covers 5,000 generated 32-dimensional vectors under conservative
debug-build budgets. The ignored one-million-record harness validates the stated
design envelope but must run on a controlled capacity host before a release that
claims measured one-million-vector performance. Record hardware, dimensions,
latency percentiles, RSS, disk consumption, and recall against exact search.

The current implementation rebuilds USearch after every changed batch and
exhaustively post-filters accelerated results. Production ingestion should use
large bounded batches until incremental sidecar replay is implemented.
