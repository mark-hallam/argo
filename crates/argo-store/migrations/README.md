# Raw SQL migration slot corpus

This directory is the raw SQL slot-collision corpus for
`crates/argo-store/build.rs`.

The files here use `NN__name.sql` filenames so build-time and runtime
validators can detect duplicate numeric slots. The highest `NN__`
prefix in this directory is only a slot number. It is not the
`schema_version` value, and it does not control which migrations the
running binary applies at startup.

The schema-apply path lives in the sibling Rust migration directory:
`crates/argo-store/src/migrations/`.

Use that sibling directory when answering questions like:

- Which migration version is the binary expected to apply?
- What value should `EXPECTED_SCHEMA_VERSION` match?
- What migration tip is used by the startup runner?

Use this directory when answering questions like:

- Did two raw SQL files reuse the same numeric slot?
- What SQL filenames are embedded into `MIGRATION_FILES` for duplicate
  slot validation?

Do not infer the runtime schema version from raw SQL slot numbers.
