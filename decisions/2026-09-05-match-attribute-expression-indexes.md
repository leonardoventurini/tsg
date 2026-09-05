# Match registered SQLite attribute expression indexes

## Context

SCS retained-symbol resolution already memoizes misses, but its TSG attribute
lookup still scanned the full node store for every distinct unresolved symbol.
The registered index exists; a parameterized JSON path does not match its literal
expression. SQLite documents this syntactic requirement:
https://www.sqlite.org/expridx.html

## Decision

Build lookup SQL through a private helper that validates the existing restricted
JSON path grammar before emitting a literal path expression. Scoped calls use
direct scope equality; unscoped calls preserve existing global semantics. Bind
all values, scope IDs, limits, and offsets. Preserve ID ordering and JSON equality.

## Alternatives and consequences

Adding another index does not repair the expression mismatch. Removing path
validation would introduce SQL injection risk. Broad SCS caching would duplicate
store state while leaving other callers slow. Fix the owning store query instead.
No public API, schema, index definition, dependency, or migration changes.

A query-plan regression verifies the actual private query used by the public API
selects the attribute index for scoped seeks. Generated fixtures cover nested
attributes, two scopes, absent values, pagination, and rejected malicious paths.
Unscoped searches can still scan; accelerating them is outside this bounded fix.
Release and SCS upgrade follow separately; code rollback requires no data rollback.
