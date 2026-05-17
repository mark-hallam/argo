# SQL interpolation policy

SKYCOT production SQL must use prepared statements for data values.
The default shape is:

```rust
sqlx::query("SELECT id FROM worker WHERE tenant_id = ?").bind(tenant_id)
```

Do not build SQL with `format!()` when any interpolated value can come
from an operator, tenant, worker, request, file, environment variable,
or provider response.

## Required audit

Run this before opening a PR that touches store or audit query code:

```bash
bash scripts/check-prepared-statements.sh
```

Green output is `count: 0 undocumented SQL interpolation site(s)`.
CI runs the same script in the `prepared-statements` job.

## Finding categories

- RED: production `format!()` SQL with user-controlled data. Convert
  values to `?` binds or a `QueryBuilder` with `.push_bind(...)`.
- YELLOW: dynamic table, column, WHERE-fragment, or PRAGMA text that
  SQLite cannot bind. Keep it only behind a fixed allowlist, enum,
  static table inventory, or validator.
- GREEN: tests, fixtures, and `.sql` migration files. They may use
  literal SQL interpolation when no runtime user data is involved.

## YELLOW exception rule

Every YELLOW production site must carry a nearby comment:

```rust
// audit-allow: identifier substitution from SomeEnum allowlist
```

The comment is not the control. The control is the adjacent match,
static inventory, validated hex literal, or other fixed gate that
prevents arbitrary SQL text from crossing the boundary. The comment
lets the script fail closed when a future raw interpolation site is
added without review.

## Reviewer checklist

- Values are bound with `?`, not interpolated.
- Any dynamic identifier is sourced from a finite list.
- Any SQLCipher or SQLite PRAGMA literal is validated before format.
- The PR body lists the audit findings and whether each was RED,
  YELLOW, or GREEN.
