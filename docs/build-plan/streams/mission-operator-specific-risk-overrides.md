# Mission load-bearing wire — Operator-specific risk overrides

**Owner**: claude. **Depends on**: existing `argo-channels::operator_preferences` (TODO brief; if not yet merged, fold this brief's concerns into that one — verify via `rg -l 'OperatorPreferences\|operator_preferences' crates/argo-channels/src/`), existing `argo-types::RiskLevel` (verify via `rg 'pub enum RiskLevel' crates/argo-types/src/`).
**Authored against**: origin/main `694a50ee` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-operator-specific-risk-overrides`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #122 ("Add operator-specific risk overrides"). **Tier M (claim 6 governance).** Today the system computes risk-level for every action via a global policy (default mapping defined in capability + vertical config). Some operators are paranoid (treat-all-as-Critical), some are looser (`book_travel` is Low for me). The system has no seam for that. This brief lands the override seam so operators can pin `(action_kind, risk_level)` overrides without forking the global policy.

## Scope

NEW `crates/argo-channels/src/operator_risk_overrides.rs` (≤ 240 LOC) — co-located with `operator_preferences` since they're the same seam at the same dispatch point:

```rust
pub struct OperatorRiskOverrides {
    pub operator_id: OperatorId,
    /// (action_kind, vertical_id) -> risk level override
    /// vertical_id == None means "all verticals for this operator"
    pub overrides: HashMap<(String, Option<String>), RiskLevel>,
    /// Cap on how loose an override can go — operator can't pin "Critical → Low" without explicit acknowledgement
    pub allow_downgrade_below_default: bool,
}

pub trait OperatorRiskOverridesSource: Send + Sync {
    async fn for_operator(&self, op: &OperatorId) -> ArgoResult<OperatorRiskOverrides>;
}

pub struct RiskResolver {
    overrides_source: Arc<dyn OperatorRiskOverridesSource>,
    default_policy: Arc<dyn DefaultRiskPolicy>, // existing capability/vertical-driven default
}
impl RiskResolver {
    pub fn new(...) -> Self;
    pub async fn resolve(&self, op: &OperatorId, action_kind: &str, vertical_id: &str) -> ArgoResult<(RiskLevel, RiskResolutionTrace)>;
}

pub struct RiskResolutionTrace {
    pub default: RiskLevel,
    pub override_applied: Option<RiskLevel>,
    pub override_scope: Option<OverrideScope>, // PerVertical(id) | AllVerticals
    pub downgrade_blocked: Option<RiskLevel>, // Some(would_have_been) if override refused
}
```

Wired with two `OperatorRiskOverridesSource` impls in this brief:
1. **`TomlOperatorRiskOverridesSource`** — reads `[operator.<id>.risk_overrides]` blocks from `argo.toml`. Mtime-poll reload (5s cache, same pattern as operator_preferences).
2. **`InMemoryOperatorRiskOverridesSource`** — test-only.

Dispatch wiring: the action-classification site that today calls `default_policy.classify(action_kind, vertical_id)` is updated to call `RiskResolver::resolve(operator_id, action_kind, vertical_id)` instead. Verify the call site via `rg -n 'classify.*risk\|RiskLevel::' crates/argo-cli/src/handlers/worker_actions*.rs crates/argo-runtime/src/`.

NEW `argo operator risk-overrides set/get/list/delete` CLI surface (`crates/argo-cli/src/handlers/operator_risk_overrides.rs` ≤ 180 LOC):
- `set <operator-id> <action-kind> <risk-level> [--vertical <id>] [--allow-downgrade]`
- `get <operator-id> [--json]`
- `list [--json]`
- `delete <operator-id> <action-kind> [--vertical <id>]`

Stores in NEW `operator_risk_overrides` table (migration v59+ — verify next-free slot; v58 reserved by mission-audit-chain-watchdog, v57 reserved by mission-gateway-request-audit-log).

This is NOT in scope: tenant-level risk overrides (separate brief — different scope); per-vertical-template default risk maps (separate); per-action-instance risk pinning (e.g. "this specific send_email is High because the recipient is the CEO" — that's a runtime-context concern, separate); risk-override audit trail beyond the audit-chain row that already records every action's `(action_kind, risk_level, operator_id)`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-channels/src/operator_risk_overrides.rs` (NEW, ≤ 240 LOC)
- `crates/argo-channels/src/operator_risk_overrides_tests.rs` (NEW, sibling, ≤ 280 LOC) — 9 unit tests:
  - no-override-returns-default
  - per-vertical-override-applied
  - all-verticals-override-applied
  - per-vertical-overrides-all-verticals-when-both-present (per-vertical wins)
  - downgrade-blocked-without-allow-downgrade
  - downgrade-allowed-with-allow-downgrade
  - upgrade-always-allowed (paranoia is free)
  - trace-shape-when-override-applied
  - trace-shape-when-downgrade-blocked
- `crates/argo-channels/src/lib.rs` — narrowly scoped: register module + re-exports
- `crates/argo-store/src/operator_risk_overrides.rs` (NEW, ≤ 180 LOC) — store primitives
- `crates/argo-store/src/operator_risk_overrides_tests.rs` (NEW, sibling, ≤ 150 LOC) — 5 unit tests
- `crates/argo-store/src/lib.rs` — narrowly scoped: register
- `crates/argo-store/src/migrations/vNN.rs` (NEW) — verify next-free slot (v59+ at brief author time)
- `crates/argo-store/migrations/<NN>__operator_risk_overrides.sql` (NEW, ≤ 30 LOC)
- `crates/argo-store/src/migrations/mod.rs` — narrowly scoped: register
- `crates/argo-cli/src/handlers/operator_risk_overrides.rs` (NEW, ≤ 180 LOC)
- `crates/argo-cli/src/cli_operator_cmds.rs` (NEW or extend if exists; ≤ 100 LOC) — `OperatorCmd::RiskOverrides { ... }` subcommand tree
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: register
- ONE dispatch-site rewrite at the action-classification call site (verify exact path at claim-time): narrowly scoped change to call `RiskResolver::resolve` instead of the default-policy directly
- `crates/argo-cli/tests/operator_risk_overrides_smoke.rs` (NEW, ≤ 180 LOC) — integration smoke: `argo operator risk-overrides set X book_travel low --allow-downgrade` → trigger a `book_travel` action as operator X → assert the audit row's `risk_level = "low"`, NOT the default

## Files you must NOT touch

- `crates/argo-types::RiskLevel` definition — stable, reuse
- `crates/argo-channels/src/{slack,email,sms,voice,whatsapp,discord,pagerduty}.rs` — channel impls
- Any vertical content
- Frontend

## Acceptance criteria

- [ ] `argo operator risk-overrides set OP123 send_email_external critical` → next `send_email_external` by OP123 is classified Critical regardless of default
- [ ] `argo operator risk-overrides set OP123 book_travel low --allow-downgrade` → next `book_travel` by OP123 is Low (would have been Medium); without `--allow-downgrade` flag, override is REFUSED with a clear error
- [ ] Upgrade-only is the default — paranoia is free, looseness requires explicit `--allow-downgrade`
- [ ] Per-vertical override wins over per-operator-all-verticals when both are set
- [ ] TOML reload on mtime change works (5s cache, same pattern as operator_preferences)
- [ ] `RiskResolutionTrace` is surfaced into the audit-chain row so operators can see WHY an action got its risk classification
- [ ] 9+5 unit tests + 1 integration smoke
- [ ] `cargo clippy -p argo-channels -p argo-store -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-channels -p argo-store -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-operator-specific-risk-overrides`
- PR title: `governance: operator-specific risk overrides (per-vertical + all-verticals scopes, upgrade-only by default) (mission-operator-specific-risk-overrides)`

## Notes for the implementer

- The `--allow-downgrade` gate is LOAD-BEARING — without it, an attacker who compromises an operator account could systematically downgrade every action to Low and bypass the approval surface. The default of "upgrade-always, downgrade-requires-explicit-flag" makes the attack chain require BOTH the account AND the explicit flag-acknowledgement.
- Surfacing `RiskResolutionTrace` into the audit-chain row is also load-bearing — forensics needs to know "this send_email was Low because OP123 set an override, not because the default policy said Low." Persist the trace JSON in a new audit-chain column or in the existing JSON-payload field.
- Per-vertical override winning over all-verticals: this is the principle-of-least-surprise direction. An operator who sets `book_travel = Low` globally but `book_travel = Medium for legal-paralegal` should see Medium when they're in legal context. Document this precedence explicitly.
- Migration slot coordination: at brief-author time, v55 is shipped, v56 is in flight (per-tenant-rate-limiting #1338 MERGED), v57 reserved by mission-gateway-request-audit-log, v58 reserved by mission-audit-chain-watchdog. This brief takes v59. Verify with `ls crates/argo-store/migrations/ | sort -V | tail -10` at claim time.
- The dispatch call site is the trickiest part — find the ONE place where today's code reads `RiskLevel` for an action and route it through `RiskResolver::resolve`. If there are multiple sites, list them in the PR body's "Sensible deviations" — but pick one canonical site and route the others through follow-up briefs.
- If `mission-operator-preferences-resolver` ships first, mirror that brief's `OperatorPreferencesSource` trait shape exactly — same `TomlPreferencesSource` mtime-poll pattern, same `InMemoryPreferencesSource` test shape. Stay consistent across the two `argo-channels::operator_*` modules.
