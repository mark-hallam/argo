# Mission Phase 2 — SOC2-aligned audit trail report

**Owner**: claude. **Depends on**: none (reads existing audit table).

## Scope

SOC2 Common Criteria 6.x ("Logical and Physical Access Controls") requires
demonstrable evidence of who did what, when, with what outcome. Argo already
records audit rows; this brief adds a CLI that emits a SOC2-shaped report:

```
argo compliance soc2 \
  --from <rfc3339> --to <rfc3339> \
  [--actor <id>] [--action <kind>] [--tenant <id>] \
  [--format json|csv|markdown]
```

The report groups events by control category (Access, ChangeMgmt, DataMgmt,
Security) using a static action-to-category map, computes per-category counts
+ first/last timestamps, and lists every event in the window with full row
detail. Output is suitable for handing to an external auditor.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/compliance_soc2.rs` (~280 LOC):
   - `pub async fn run(args: ComplianceSoc2Args, store: &Store) -> ArgoResult<i32>`
   - `static ACTION_CATEGORY: &[(&str, Soc2Category)]` mapping audit actions
     to categories:
     - `auth.login`, `auth.logout`, `tenant.role_change` → `Access`
     - `worker.deploy`, `manifest.publish`, `executor.update` → `ChangeMgmt`
     - `compliance.export`, `compliance.delete`, `data.import` → `DataMgmt`
     - `policy.update`, `secret.rotate`, `audit.verify_chain` → `Security`
     - default unknown → `Other`
   - Filters audit table by `[from, to]` (+ optional actor/action/tenant)
   - Computes per-category aggregates
   - Emits `--format json` (raw structured), `--format csv` (per-event flat),
     `--format markdown` (human-readable summary + table), defaulting to
     markdown for terminal-friendliness
2. **Modify** `cli.rs` + `main.rs`:
   - New `Compliance::Soc2` sub-subcommand (sibling of Export + Delete)

## Manifest contract / wire format

CLI surface only. JSON output shape:

```json
{
  "window": { "from": "...", "to": "..." },
  "filters": { "actor": null, "action": null, "tenant": null },
  "categories": {
    "Access": { "count": 12, "first": "...", "last": "..." },
    "ChangeMgmt": { "count": 4, "first": "...", "last": "..." }
  },
  "events": [
    { "id": 1, "ts": "...", "actor": "u-1", "action": "auth.login",
      "target": "...", "category": "Access" }
  ]
}
```

## Files you may touch

- `crates/argo-cli/src/handlers/compliance_soc2.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/compliance_soc2_tests.rs` (NEW, ≤ 250 LOC) —
  7 unit tests:
  - empty window → valid empty report
  - 1 event of each category → category counts correct
  - actor filter → only matching events returned
  - action filter → only matching events returned
  - tenant filter → only matching events returned
  - markdown format renders without panic and includes all categories
  - csv format has stable column order
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line
- `crates/argo-cli/src/cli.rs` — narrowly: `Compliance::Soc2` subcommand
- `crates/argo-cli/src/main.rs` — narrowly: 1 dispatch arm
- `crates/argo-cli/tests/compliance_soc2_smoke.rs` (NEW, ≤ 200 LOC) —
  1 integration test seeding mixed events and asserting category aggregates

## Files you must NOT touch

- Audit-write paths — this brief is read-only over audit
- The action-name registry elsewhere — `ACTION_CATEGORY` is a separate
  classification-only mapping; if a new action name appears it falls into
  `Other` until explicitly classified

## Acceptance criteria

- [ ] Window filter inclusive on `from`, exclusive on `to` (documented)
- [ ] Optional actor/action/tenant filters AND together
- [ ] Category aggregates correct
- [ ] All three formats (json/csv/markdown) emit valid output
- [ ] Markdown format includes category summary + per-event table
- [ ] 7 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-compliance-soc2-audit-trail`
- PR title: `compliance: SOC2 audit trail report (claude-compliance-soc2-audit-trail)`

## Notes for the implementer

- The category map is intentionally explicit — keep `Other` as the default so
  unclassified actions are visible in the report rather than silently lost.
- For markdown format, generate a sectioned report (one `##` per category)
  followed by a single events table. Keep the table column count under 8 for
  readability in narrow terminals.
- Reuse any existing `argo-cli` markdown helpers (`crates/argo-cli/src/...`
  has markdown-rendering helpers — search before reimplementing).
