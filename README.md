# TSG

TSG is a transactional semantic graph storage engine. SQLite is the durable
authority for graph records and canonical embeddings; exact and USearch vector
indexes are query accelerators derived from that state.

The initial crate is an internal foundation for SCS, not a general-purpose
graph database or a stable public API.

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

