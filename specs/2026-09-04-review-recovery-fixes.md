# Review and recovery fixes

## Problem, evidence, and uncertainty

Review the storage boundary consumed by SCS before releasing a patch. Writable
adaptive search currently retries an unavailable sidecar and errors instead of
reading canonical vectors. Scope changes need review against retained edges.
The existing lifecycle suite covers read-only fallback, but not writable fallback.
This is a focused correctness review, not a capacity certification.

## Contracts and constraints

Preserve public signatures, schema version 2, SQLite authority, and explicit
USearch error reporting. Adaptive search must remain available after sidecar
failure. Every committed edge must remain within one scope, including node-only
updates. Release through the existing GitHub tag workflow as v0.2.1, then upgrade
SCS. No new production dependencies.

## Risks and recovery

Exact fallback may be slower than ANN but preserves availability. Reject invalid
scope transitions atomically. Existing invalid data is not silently rewritten.
Rollback consumers to v0.2.0 if needed; these fixes require no data migration.

## Executable checklist and direct rollout

- [x] Add and run generated regression fixtures before fixes.
- [x] Fix confirmed storage/search defects; document their limits.
- [x] Run fmt, clippy, all-target tests, rustdoc, coverage, and packaging.
- [x] Record decisions and commit task-owned paths with hooks enabled.
- [x] Bump patch version and publish v0.2.1 through the existing release workflow.
- [x] Confirm release and artifacts before upgrading SCS.

## Verification

Regression tests must prove exact fallback with unavailable sidecar, explicit
USearch failure, and transaction rollback when a node update would create a
cross-scope edge. Existing tests retain reopen, deletion, and performance gates.

Executed: 38 tests passed, the opt-in million-vector harness remained ignored,
and line coverage was 87.65% (85% required). Formatting, strict Clippy, rustdoc,
and package build/verification passed. cargo-deny passed with existing unmatched
license allowance and duplicate transitive version warnings. Literal name lookup
regressions also failed before and passed after the fix. No schema migration or
public signature changes were needed.

Release commit: `6aa395c246ab0342342662387dfdffe7dbea0be3`. Both CI runs and
[release workflow](https://github.com/leonardoventurini/tsg/actions/runs/33940322824)
succeeded. The downloaded archive matched its published SHA256SUMS entry:
`cdfb0c261ad9b0fba862f4c1e8bcd50325f984c8e8f261a34127c40646e2a0ad`.
The release run had nonblocking cache-cleanup ENOENT annotations for absent test
directories; package build and publication completed successfully.
