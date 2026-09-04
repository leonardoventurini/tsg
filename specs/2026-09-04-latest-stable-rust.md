# Latest stable Rust and dependencies

Project: `tsg`

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

## Problem and evidence

TSG 0.1.0 declares Rust 1.83 support, but its permitted `usearch` dependency
range resolves to packages with Rust 2024 manifests. Cargo 1.83 fails before
compilation on both macOS and Linux. Stable Rust, coverage, and supply-chain jobs
pass, demonstrating a toolchain compatibility failure rather than platform or
runtime behavior.

## Desired outcome

Adopt the current stable Rust release, Rust 1.98, the Rust 2024 edition, and the
latest available direct dependencies and GitHub Actions. Test stable Rust on
macOS and Linux and use the committed lockfile consistently in CI. Publish the
correction as TSG 0.1.1.

## Scope and contracts

- Set `package.rust-version` to `1.98` and `edition` to `2024`.
- Use the stable toolchain on both macOS and Linux.
- Update every direct dependency and GitHub Action to the latest available
  release at implementation time.
- Use `--locked` for dependency-resolving CI checks.
- Preserve stable-toolchain jobs and the macOS/Linux matrix.
- Update compatibility, installation, release history, and decision records.
- Do not pin transitive `cxx` packages solely for an older compiler.

## Risks and recovery

Consumers on older Rust releases cannot upgrade to TSG 0.1.1. They may retain
0.1.0 with an externally constrained dependency graph, but that release's stated
MSRV is not reliable. A major dependency update may require source adaptation.
Rollback is a commit revert; published version tags and release artifacts remain
immutable.

## Direct rollout

Verify locally with Rust 1.98 stable, commit and push the correction, create tag
`v0.1.1`, and observe both CI and release publication.

## Executable checklist

- [x] Update crate version, edition, and toolchain metadata.
- [x] Update direct dependencies and GitHub Actions.
- [x] Update CI toolchains and locked commands.
- [x] Update README and changelog.
- [x] Verify formatting, lints, tests, docs, packaging, and workflow syntax.
- [x] Commit and push.
- [x] Tag and publish TSG 0.1.1.
- [x] Confirm all CI jobs and release assets.
