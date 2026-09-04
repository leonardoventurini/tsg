# Contributing

Changes must preserve SQLite as the durable authority and keep USearch
rebuildable from canonical embeddings.

Before submitting a change, run:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo llvm-cov --all-targets --all-features --fail-under-lines 85
cargo package --allow-dirty
```

Behavior changes require tests. Storage changes require migration, recovery,
and backward-compatibility coverage. Do not weaken durability or application-scope
isolation silently.
