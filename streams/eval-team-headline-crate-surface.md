# eval-team-headline-crate-surface

**Priority**: P3 (claim-1 substrate audit — last unaudited Claim 1 crate)
**Owner**: claude
**Type**: Read-only audit + regression smoke
**Size**: ~80-200 LOC: one Markdown audit report + one structural pin test
**Authored against**: origin/main `01bbe9b58` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-team-headline-crate-surface`

## Why

The gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` Claim 1 row:

> `argo-team-headline` | ? | 🔴 NOT AUDITED | Crate exists, unknown surface. Worker overview page uses it per `routes_team_list.rs` import. | `routes_team_list.rs:31` | 🟢 P3 | `eval-team-headline-crate-surface` — read + assess.

Claim 1 ("Teammate, not tool") is in excellent shape — `argo-identity` is 🟢 (rounds 2+7), `argo-memory` is 🟢 (round-13 + this week's anti-theater verifies). The team-headline crate is the **only remaining 🔴 NOT AUDITED** cell. It's a P3 because the failure mode is "operator sees stale headline on team page" — not data-loss or cross-tenant leak — but closing the audit cell tightens the v1.0 ship-gate evidence chain.

This brief reads the crate, writes a one-page audit report, and pins one structural test that fails if the crate's public surface drifts from the audit verdict.

## What

### Step 1 — read

`crates/argo-team-headline/src/lib.rs` and any siblings. Build a mental model of:
- public surface (`pub fn`/`pub struct`/`pub enum`),
- workspace consumers (`rg 'argo-team-headline = ' crates/*/Cargo.toml`),
- callsites (`rg 'argo_team_headline::' crates/`),
- test coverage (`ls crates/argo-team-headline/tests/`).

### Step 2 — audit report

New file `docs/skycot/audit/2026-05-24-argo-team-headline-crate-surface/audit.md` (~60-120 LOC). Sections:

1. **Surface** — every `pub` symbol with a one-line description.
2. **Consumers** — which crates import it, and at which callsites.
3. **Tenant scoping** — does the crate respect `tenant_id`? `rg 'tenant' crates/argo-team-headline/` to confirm. (Claim 1 substrate must be multi-tenant safe per MISSION.md invariant #2.)
4. **Test coverage** — what's pinned, what's not.
5. **Verdict** — 🟢/🟡/🔴 cell for the gap manifest. Recommended fix-briefs only if 🔴 verdict.

### Step 3 — structural pin test

New file `crates/argo-team-headline/tests/surface_pin_test.rs` (~30-80 LOC). One test that:

- enumerates the `pub` symbols matching the audit report's §1 list (use `const _: fn() = || { let _: ... = ...; }` patterns to assert signatures compile),
- asserts the crate compiles standalone,
- asserts test fixtures exist for any public type the consumer crate (`argo-cli` via `routes_team_list.rs`) actually constructs.

If the audit reveals zero workspace consumers (dead crate), the pin test instead asserts `rg 'argo-team-headline = ' crates/*/Cargo.toml` returns ≥1 result — drift-tripping if a consumer drops the dependency. (Use a `cargo metadata`-based test, not a shell-out; example pattern at `crates/argo-mcp/tests/connector_inventory_pin_test.rs`.)

## Acceptance

- [ ] `docs/skycot/audit/2026-05-24-argo-team-headline-crate-surface/audit.md` exists with the 5 sections.
- [ ] `crates/argo-team-headline/tests/surface_pin_test.rs` exists and passes on origin/main.
- [ ] `cargo clippy -p argo-team-headline --all-targets -- -D warnings` clean.
- [ ] Verdict in the audit report is 🟢, 🟡, or 🔴 — never "TBD."
- [ ] If 🔴, the report names follow-up fix-briefs (does NOT author them).
- [ ] No production `.rs` file under `crates/argo-team-headline/src/` is touched.

## May-touch

- `docs/skycot/audit/2026-05-24-argo-team-headline-crate-surface/audit.md` (NEW)
- `crates/argo-team-headline/tests/surface_pin_test.rs` (NEW)
- `crates/argo-team-headline/Cargo.toml` (only if `[dev-dependencies]` needs a test helper)

**Out of may-touch**: every other crate; every `.rs` file under `crates/argo-team-headline/src/`.

## Parallel surfaces

- `rg 'team_headline.*pin' crates/` returns no hits — net-new test file.
- The structural-pin idiom is borrowed from `crates/argo-mcp/tests/connector_inventory_pin_test.rs` — reuse the pattern, do not duplicate the test.

## Sensible deviations

- If the crate is <100 LOC and trivially shaped (just a `pub struct TeamHeadline { fields }` and a builder), the audit report compresses §1-3 into a one-paragraph "crate is a passive data carrier" verdict. Don't pad. Smaller is better.
- If `argo-team-headline` is structurally a candidate for collapse into `argo-team-list` or `argo-identity`, name that recommendation in the audit report under "follow-up briefs." Do NOT author the consolidation in this brief — that's a separate brief class.
- If the test fixture needs a Postgres / SQLite spin-up to construct a `TeamHeadline`, skip that test path and document the gap. Audit-only smoke tests should not require infrastructure.
