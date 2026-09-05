# Update vector accelerator incrementally after committed batches

## Context

Repeated 32-vector ingestion batches rebuilt all historical ANN links, dominating
real consumer latency despite fast model responses. SQLite already owns every
vector and monotonically increasing committed generation; USearch supports key
removal and insertion without rebuilding unrelated vectors.

## Decision

Capture the authoritative generation before mutation within the SQL transaction.
After successful commit, a matching in-memory accelerator applies changed keys
and persists using the existing sidecar protocol. Replacements remove then add;
metadata-only batches retain the ANN structure. Missing or stale accelerators
continue rebuilding from SQLite. On any incremental mutation/persistence failure,
discard the accelerator and report `accelerator_ready = false`; never expose its
partial state or turn an already committed SQL write into a failed receipt.

## Rationale and consequences

This removes cumulative ANN reconstruction without changing APIs, dependencies,
storage format, or crash recovery. Full sidecar serialization still occurs for
each commit; asynchronous durability or a new storage protocol is outside scope.
Keeping a partial index after failed replacement was rejected because it could
silently omit committed vectors. Eager full repair after an incremental failure
was rejected because it obscures degraded readiness and repeats expensive work.

Generated tests pin instance reuse, replacement uniqueness, transactional rollback,
stale/missing generation recovery, failure fallback, and read-only reopen. An
opt-in 4,096-dimensional benchmark supplies reproducible diagnostics without
machine-dependent CI timing ceilings. Release and consumer verification are
coordinated separately after required local checks.
