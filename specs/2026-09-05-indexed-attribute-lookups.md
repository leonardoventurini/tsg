# Indexed attribute lookup query planning

## Problem and evidence

SCS ingestion spends most sampled CPU time resolving retained qualified names.
SCS already caches positive and negative lookups and registers a qualified-name
expression index. TSG binds the JSON path as a parameter and uses an optional
scope OR predicate, so SQLite scans nodes instead of seeking the existing index.
An isolated EXPLAIN reproduces this behavior.

## Contracts and uncertainty

Preserve the public API, SQLite schema, JSON value semantics, deterministic ID
ordering, pagination, and unscoped lookup behavior. Interpolate only paths that
pass the existing restrictive path validator; values remain bound parameters.
Scoped lookup must use the existing composite attribute index. Unscoped lookup
can still scan because that index starts with scope. No dependencies change.

## Tests and rollout checklist

- [x] Reproduce an indexed scoped lookup query-plan failure before fixing SQL.
- [x] Test generated multi-scope hits/misses, ordering, pagination, nested paths,
      and malicious paths through the public lookup API.
- [x] Use literal validated paths and separate scoped/unscoped query predicates.
- [x] Run format, strict Clippy, tests, rustdoc, package, coverage, dependency audit.
- [x] Commit verified changes and record decision; release separately.

## Risks and recovery

Literal SQL construction must remain behind path validation; arbitrary values
must never be interpolated. Derived expression indexes already exist, so no
migration or data rollback is needed. Reverting code restores previous lookup
behavior at the cost of scans. Version bump, publication, and SCS consumer upgrade were completed after
validation as recorded below.

## Executed verification

The query-plan test failed before the SQL fix with a node-ID index scan; it now
requires an attribute-index seek on scope and expression. Generated 2,048-node
functional fixtures passed before and after. Format, strict Clippy, all-target
all-feature tests (40 passed; one opt-in million-vector harness ignored), strict
rustdoc, package build/verification, and coverage passed. Line coverage: 89.03%
against 85%. cargo-deny passed advisories, bans, licenses, and sources, with existing
duplicate transitive-version and unused license-allowance warnings. No production
benchmark is claimed; the deterministic query plan proves index eligibility.
Published immutable TSG v0.2.2 at commit
`6e6e607ed80704b4169ed52c5217e76d9a36196a`. GitHub CI `33981102333`
passed macOS/Linux verification, coverage, and supply-chain checks. Release run
`33981318370` passed and published `tsg-0.2.2.crate` and `SHA256SUMS`.
The downloaded source archive matched its published SHA-256:
`9d3f9b77919aac86fc1ae83d3e7eeb79d5e400d6326d5b307652376a3a4b6724`.
SCS consumes the immutable Git tag starting with its published 0.1.5 release.
Release: https://github.com/leonardoventurini/tsg/releases/tag/v0.2.2
