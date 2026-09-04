# GitHub release publishing

## Problem

Project: TSG

Project root: `/Users/leonardo/Repositories/leonardoventurini/tsg`

Tag pushes currently create short-lived workflow artifacts on two operating
systems. They do not create a durable GitHub Release, and the README does not
explain version-tag or release-archive installation.

## Evidence and uncertainty

- `.github/workflows/release.yml` triggers on `v*` tags.
- The current job packages the same platform-independent Rust source crate on
  macOS and Linux.
- No GitHub Release or repository package currently exists.
- TSG remains a private, unpublished pre-1.0 crate.

GitHub Releases are not a Cargo registry. Consumers can use a private Git tag
directly or download and unpack the attached crate as a vendored path dependency.

## Contracts

- A pushed release tag must equal `v` followed by the version in `Cargo.toml`.
- Release validation must run the locked full test suite and `cargo package`.
- A successful tag workflow must create a GitHub Release containing the
  platform-independent `.crate` archive and a SHA-256 checksum.
- The workflow must use only the repository-scoped token with `contents: write`.
- README installation instructions must distinguish path, private Git tag, Git
  revision, and downloaded-release workflows.
- This task does not publish to crates.io or GitHub Packages.

## Risks

- A mismatched tag could publish misleading version metadata.
- Private consumers require explicit GitHub authentication.
- Re-running a completed tag workflow cannot create the same release twice.

## Recovery

If publication fails, correct the workflow on `main`, delete the failed release
only if one was partially created, and rerun the tag workflow. The Git tag and
crate version remain the release identity. Deleting or moving a successfully
published version tag is outside the normal recovery path.

## Direct rollout

Commit the workflow, README, specification, and decision record together. Push
`main`, create signed-off lightweight tag `v0.1.0` at that commit, push the
tag, and observe the workflow through successful release creation.

## Executable checklist

- [x] Replace matrix artifact uploads with one release job.
- [x] Validate tag/version equality.
- [x] Test and package with `--locked`.
- [x] Generate and attach `SHA256SUMS`.
- [x] Document supported installation methods and authentication.
- [x] Validate README doctests, strict documentation, workflow syntax, tests,
      and package assembly.
- [x] Commit and push `main`.
- [x] Tag and push `v0.1.0`.
- [x] Confirm the GitHub Release and attached assets.

## Verification

Local verification covers the package and documentation. GitHub Actions is the
authoritative verification for token permissions and release creation.
