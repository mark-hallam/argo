# Runtime schema-apply migrations

This directory is the runtime schema-apply path for `argo-store`.

The `vN.rs` modules here are registered by the Rust migration runner
and applied by the running binary at startup. The highest registered
`vN` module is the schema tip, and `crates/argo-store/src/lib.rs`
`EXPECTED_SCHEMA_VERSION` must match that tip.

The sibling raw SQL directory, `crates/argo-store/migrations/`, has a
different purpose. Its `NN__name.sql` files are read by
`crates/argo-store/build.rs` and embedded for duplicate slot validation.
Those `NN__` prefixes are slot-collision bookkeeping, not runtime
schema versions.

Use this directory when answering questions like:

- What schema version should a binary expect?
- Which migrations does startup apply?
- Which tip should `EXPECTED_SCHEMA_VERSION` match?

Use the sibling `migrations/` directory only for raw SQL slot-collision
validation. Do not compare its highest `NN__` prefix to
`EXPECTED_SCHEMA_VERSION`.
