# Eval Fixture Versioning

Eval fixtures are JSONL rows under `verticals/*/eval/` and
`packs/*/evals/`. The current schema version is `1`.

## Policy

- `schema_version` is optional for v1 fixtures.
- Missing `schema_version` is treated as `1` for backward
  compatibility.
- Explicit `"schema_version": 1` is accepted.
- Future versions must fail closed until a compatibility shim lands.

This keeps older shipped fixtures loadable while preventing newer
fixture shapes from being silently interpreted by older scorers.

## Migration

When introducing a v2 fixture shape:

1. Add the parser/scorer compatibility code first.
2. Add tests proving v1 and v2 rows are both handled deliberately.
3. Update fixture files only after the compatibility code ships.
4. Bump this document with the new version contract.

Do not edit existing fixtures just to add `schema_version`; v1 is the
implicit default.
