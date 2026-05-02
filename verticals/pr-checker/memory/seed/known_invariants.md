# Known invariants

Seed memory for `key=known_invariants`. The PR-Checker reads this
on first run and writes new ones as it learns from human-resolved
disputes.

- No file > 500 LOC (source AND tests). Extract a sibling module.
- No `unwrap()` / `expect()` outside `#[cfg(test)]`.
- No `unsafe` Rust outside marked FFI shims.
- Every public Rust API has rustdoc with at least one example.
- No global mutable state. `AgentDeps` is the only injection point.
- Migrations are additive only. No `DROP COLUMN` / `DROP TABLE`
  in `up` blocks (down blocks are fine).
- `tenant_id` plumbed through every primitive that crosses a
  multi-tenant boundary.
- Capability checks on every tool dispatch — never call
  `tool.run()` directly without going through the dispatcher.
- Provider quirks live in TOML manifests under
  `manifests/providers/`, not in Rust code.
- MCP-first integration: the 8 in-core tools are the canonical
  set; everything else is an MCP server or plugin.
