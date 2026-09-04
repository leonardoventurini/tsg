# Track latest stable Rust

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Context

TSG 0.1.0 advertised Rust 1.83, while its allowed `usearch` dependency graph
now contains Rust 2024 manifests that Cargo 1.83 cannot parse. Both minimum-
toolchain CI jobs fail before compiling TSG.

## Decision

Beginning with 0.1.1, TSG requires the current stable Rust release at the time of
publication: Rust 1.98. It adopts the Rust 2024 edition and updates all direct
dependencies and GitHub Actions to their latest available releases.
Dependency-resolving CI commands use the committed `Cargo.lock`, and CI tests
stable Rust on macOS and Linux.

## Rejected alternatives

- Pin `usearch`, `cxx`, and build-only transitive dependencies to historical
  releases solely to preserve Rust 1.83.
- Remove the MSRV jobs while continuing to advertise unverified support.
- Retain Rust 1.85, which can parse Edition 2024 manifests but cannot compile the
  current `cxx` dependency family requiring Rust 1.88.

## Rationale

At the time of this decision, TSG was new, private, and pre-1.0 with no
established older-toolchain consumers. It has since become public; the lack of
an established compatibility baseline and its pre-1.0 status still support a
latest-stable policy without artificial transitive pins.

## Consequences

Consumers need Rust 1.98 or newer for TSG 0.1.1 and later, including the current
0.2 release line. The policy may raise that floor in future releases. Lockfile
use makes repository CI and releases reproducible, while downstream libraries
still resolve dependencies according to their own lockfiles.
