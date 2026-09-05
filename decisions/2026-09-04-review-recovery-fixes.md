# Preserve availability and scope invariants in the patch release

## Context

SCS consumes TSG adaptive search and name reads. Review found writable search
could fail after a successful canonical commit if sidecar persistence failed.
Node upserts also checked scope isolation only for newly supplied edges, and
literal name searches did not escape the SQL LIKE escape character.

## Decision and rationale

Release v0.2.1 with exact fallback for every handle lacking a current accelerator,
transactional validation of retained edges after node updates, and complete
literal LIKE escaping. Preserve public signatures, schema version 2, JSON merge
semantics, and explicit USearch failure reporting. Reject invalid scope moves
rather than silently deleting edges. Connected nodes may move together when the
final batch state remains valid.

## Alternatives and consequences

Retrying sidecar persistence on every adaptive read makes canonical data
unavailable during a persistent filesystem fault. Silent edge deletion loses
application relationships. Neither is appropriate for a corrective patch.
Exact fallback can increase query latency. Existing invalid cross-scope data is
not automatically repaired. No data migration or dependency change is required;
SCS can upgrade after the release is published and roll back its pin if needed.
