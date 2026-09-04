# Generic application integration boundary

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Context

Real consumers need durable application metadata, external partitions, indexed
lookups, and lifecycle records in the same transaction as graph and embedding
state. Encoding any one consumer's domain vocabulary in TSG would make the
storage engine an application component rather than a reusable package.

## Decision

TSG 0.2 provides generic application scopes, validated JSON attributes,
caller-declared node-attribute equality indexes, and namespaced catalog records.
Graph, embedding, and catalog mutations share one SQLite transaction and durable
generation. Version 0.1 stores are not migrated in place; callers explicitly
rebuild a new 0.2 store.

## Rejected alternatives

- Add repositories, source paths, ingestion hashes, or model-provider fields to
  TSG's schema.
- Keep a separate consumer-owned SQLite catalog beside TSG.
- Permit arbitrary SQL or arbitrary unvalidated index expressions.
- Rewrite pre-1.0 stores in place despite the breaking graph model.

## Rationale

Scopes and bounded JSON documents preserve a small reusable engine contract.
Explicit indexes cover high-value equality lookups without creating a general
relational abstraction. A clean rebuild makes the breaking pre-1.0 transition
observable and recoverable.

## Consequences

Consumers translate domain types at their adapter boundary and retain their old
database until the new store passes verification. Index paths must be declared
when opening a writable store. TSG remains intentionally narrower than a raw SQL
API and does not interpret application catalog documents.
