# codex-prepared-statements-followup — sweep remaining raw-SQL callsites (P2)

**Owner**: codex. **Sizing hint**: judgment-required (find + categorize + fix), not pure mechanical.
**Depends on**: none.
**Authored against**: origin/main `de644311` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-prepared-statements-followup`.
**Source**: CI workflow `ci.yml` has a `prepared-statements` job that lints argo-store queries. Pre-launch enterprise/SOC2: no raw SQL interpolation anywhere.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — SQL injection class-of-bug elimination.
- **Reliability** (#1) — prepared statements are cached + plan-stable.

## Scope

P2. Three-step pass:

### Step 1: Audit current state

```bash
bash scripts/check-prepared-statements.sh 2>&1 | tee /tmp/prep-stmt-audit.txt
```

(If that script doesn't exist, run `rg --type rust 'format!\(.*FROM|format!\(.*UPDATE|format!\(.*INSERT|format!\(.*DELETE' crates/` to find string-interpolated SQL.)

### Step 2: Categorize findings

- **Production callsites with user-controlled data** → 🔴 must fix.
- **Test fixtures / hardcoded queries** → 🟢 acceptable; document.
- **Migration files** (`.sql`) → 🟢 acceptable; these are operator-authored.
- **Schema-discovery queries** (`PRAGMA table_info(?)`) → 🟡 SQLite-specific; mark for review.

### Step 3: Fix the 🔴 ones

For each production callsite with user-controlled data, convert to `sqlx::query!` or `?`-bound parameters. **No `format!()` should produce SQL with user input.**

If any callsites legitimately need dynamic SQL (e.g. dynamic ORDER BY column from operator config), gate them via a fixed allowlist + `assert!(allowlist.contains(col))`.

LOC: ~150-300 depending on findings.

## Files you may touch

Wherever the audit surfaces issues. Likely candidates:
- `crates/argo-store/src/*_query.rs`
- `crates/argo-store/src/migrations/*.rs` (production code; not the `.sql` files)
- `crates/argo-audit/src/*.rs`

**Always touched:**
- `docs/skycot/security/sql-policy.md` (NEW) — operator runbook documenting the policy + the audit-script command + how to handle the 🟡 dynamic-SQL exception cases. ≤60 LOC.
- `scripts/check-prepared-statements.sh` (existing or NEW) — extend or create a deterministic lint script. ≤120 LOC.

## Files you must NOT touch

- Migration `.sql` files — operator-authored.
- Test files — acceptable to interpolate in tests.
- Any non-SQL code.

## Acceptance criteria

- [ ] PR body lists every finding from the audit (file:line, category, action taken).
- [ ] Every 🔴 finding has been fixed.
- [ ] Every 🟡 finding has a documented justification + allowlist gate (or is converted to a clear test case showing the input is constant).
- [ ] `scripts/check-prepared-statements.sh` passes on the modified workspace.
- [ ] `cargo test -p argo-store -p argo-audit` passes for any touched test fixtures.
- [ ] `cargo clippy -p argo-store -p argo-audit --all-targets -- -D warnings` clean.
- [ ] Operator runbook documents the policy.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-prepared-statements-followup`
- Title: `security: sweep remaining raw-SQL interpolation callsites (codex-prepared-statements-followup)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Don't restructure the schema.** This brief is converting `format!()` → `?`, not changing queries.
- **`sqlx::query!` requires a live DB** at macro-expansion time. If the workspace uses runtime `prepare`, prefer that pattern. Check existing code for the conventional approach.
- **Dynamic ORDER BY** — `match col { "name" | "created_at" => ... }`-style allowlist is fine. Don't try to invent a generic dynamic-column system.
- **Audit script output should be `OK` / `count: 0` on green main** so CI can `exit 0` on it.
- **The CI `prepared-statements` job already runs.** If it's currently passing, this brief's audit may surface ZERO 🔴 findings (the gate is in place). In that case, the brief delivers the runbook + script polish only. PR body documents "no production 🔴 findings; policy locked in script."
