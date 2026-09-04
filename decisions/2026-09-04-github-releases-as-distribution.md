# GitHub Releases as source distribution

## Context

At the time of this decision, TSG was private and not published to crates.io.
Its existing tag workflow retained duplicate operating-system workflow artifacts
but provided no durable release. The repository became public on 2026-09-04;
the distribution design remains unchanged and no longer requires authentication.

## Decision

Project: TSG

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

Version tags matching `v<crate-version>` create a GitHub Release containing one
Cargo `.crate` source archive and its SHA-256 checksum. Cargo consumers should
prefer a pinned Git tag or revision; release archives support explicit
vendoring and inspection.

## Rejected alternatives

- GitHub Packages: GitHub does not provide a native Cargo package registry.
- crates.io publication: intentionally deferred while TSG is pre-1.0.
- Per-platform archives: a Cargo source crate is platform-independent.
- Workflow artifacts only: retention is limited and discovery is poor.

## Rationale

GitHub Releases provide durable, versioned artifacts and generated release notes
without implying crates.io availability. Tag/version validation prevents the
release name from diverging from package metadata.

## Consequences

The release workflow needs repository contents write permission. Consumers can
access the repository and releases anonymously. Each release tag is immutable
operationally and must be created only after the version commit is ready.
