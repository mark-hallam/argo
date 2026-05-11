# Mission load-bearing wire — hard $$$ ceilings per session/day/month

**Owner**: claude. **Depends on**: `argo-store/src/spend.rs` (verified — provides `record_spend` and `sum_spend_window`); `argo-types/src/budget.rs` (verified — `Budget`, `BudgetKind`); existing per-subtask budget tree (#1315 SHIPPED via `mission-delegation-budget-tree`); existing skill budget aggregation (`argo-cli/tests/budget_alerts_smoke.rs`, `budget_csv_smoke.rs`, `budget_by_skill_smoke.rs`).
**Authored against**: origin/main `d7c20e4d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-llm-budget-hard-ceiling`.

> SOURCE: MISSION_BACKLOG.md Section 7 #225 ("Hard $$$ ceilings per session/day/month at enforcement time"). Compliance/governance gap: today `argo-store::spend` records all LLM/sandbox/connector costs and the dashboard surfaces alerts, but there is no **enforcement-time** ceiling. An operator can configure budget alerts but cannot say "stop the session if today's tenant spend exceeds $X." This brief adds per-tenant hard ceilings checked at provider-call time so over-budget calls fail clean with a typed error before incurring spend.

## Scope

NEW module `crates/argo-store/src/spend_ceiling.rs` (≤ 280 LOC) providing the read side, and NEW wrapper in `crates/argo-provider/src/spend_gate.rs` (≤ 200 LOC) providing the enforcement seam:

```rust
// argo-store::spend_ceiling
pub struct CeilingConfig {
    pub per_session_usd_micros: Option<u64>,
    pub per_day_usd_micros:     Option<u64>,
    pub per_month_usd_micros:   Option<u64>,
}
pub struct CeilingCheck {
    pub session_used_micros:  u64,
    pub day_used_micros:      u64,
    pub month_used_micros:    u64,
    pub session_remaining:    Option<u64>,    // None = uncapped
    pub day_remaining:        Option<u64>,
    pub month_remaining:      Option<u64>,
}
impl SessionStore {
    pub async fn check_ceiling(&self, tenant: &TenantId, session: &SessionId, config: &CeilingConfig) -> ArgoResult<CeilingCheck>;
}
```

```rust
// argo-provider::spend_gate
pub struct SpendGate<T: Transport> { inner: T, store: SessionStoreHandle, config: Arc<CeilingConfig> }
impl<T: Transport> SpendGate<T> {
    pub fn new(inner: T, store: SessionStoreHandle, config: CeilingConfig) -> Self;
}
#[async_trait]
impl<T: Transport> Transport for SpendGate<T> {
    async fn complete(&self, req: CompletionRequest, ctx: &TransportContext) -> Result<CompletionResponse, ProviderError> {
        // 1. Pre-call: project the request's estimated cost onto current windows.
        // 2. If any window would exceed its ceiling → return ProviderError::BudgetExceeded {
        //        window: BudgetWindow::Session|Day|Month,
        //        used_micros, ceiling_micros, projected_cost_micros
        //    } WITHOUT calling the inner transport.
        // 3. Else call inner. On success, the existing spend.record_spend (called by the caller) will accrue.
    }
}
```

The hook is wired narrowly at provider construction (mirrors `mission-circuit-breaker-wire-providers` shape — the new `from_manifest_with_budget_gate(manifest, store, config)` constructor returns `SpendGate<T>` wrapping the bare transport). Production callers (`argo-agent-runner`, `argo-cli/judge_eval`, etc.) continue using `from_manifest` unchanged; a separate routing brief migrates each callsite.

Config source: NEW `[budget.ceiling]` block in `argo.toml`:

```toml
[budget.ceiling]
# Each is optional; absent → uncapped.
per_session_usd = 5.00
per_day_usd     = 100.00
per_month_usd   = 2000.00
```

Parsed by NEW `crates/argo-config/src/budget_ceiling.rs` (≤ 100 LOC), converted to micros at load time.

This is NOT in scope: per-vertical or per-worker ceilings (sibling work item); ceiling pre-warning before the hard wall (dashboard alerts already exist); soft-cap with operator-prompt-to-continue UX (separate brief once enforcement lands); cost estimation refinement (uses existing `CompletionRequest::estimated_cost_micros` if present, else a conservative 4¢/req fallback); chargeback / invoicing surfaces.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/spend_ceiling.rs` (NEW, ≤ 280 LOC) — `CeilingConfig`, `CeilingCheck`, `check_ceiling` method on `SessionStore`
- `crates/argo-store/src/spend_ceiling_tests.rs` (NEW, sibling, ≤ 250 LOC) — 7 unit tests: uncapped-all, session-cap-only, day-cap-only, month-cap-only, all-three-set, exact-boundary, exceeds-by-1-micro
- `crates/argo-store/src/lib.rs` — narrowly scoped: ONE `pub mod spend_ceiling;` + re-export
- `crates/argo-provider/src/spend_gate.rs` (NEW, ≤ 200 LOC) — `SpendGate<T: Transport>` wrapper + new constructors
- `crates/argo-provider/src/spend_gate_tests.rs` (NEW, sibling, ≤ 180 LOC) — 5 unit tests: under-budget passes through, over-session-fails-fast, over-day-fails-fast, over-month-fails-fast, projected-cost-equal-to-remaining is allowed
- `crates/argo-provider/src/lib.rs` — narrowly scoped: ONE `pub mod spend_gate;` + re-export
- `crates/argo-config/src/budget_ceiling.rs` (NEW, ≤ 100 LOC) — `[budget.ceiling]` parser + tests
- `crates/argo-config/src/budget_ceiling_tests.rs` (NEW, sibling, ≤ 120 LOC) — 5 unit tests: all-uncapped, all-set, invalid-negative, zero-explicit, mixed-uncapped
- `crates/argo-config/src/lib.rs` — narrowly scoped: ONE `pub mod budget_ceiling;` + re-export
- `crates/argo-types/src/error.rs` — narrowly scoped: ADD `ProviderError::BudgetExceeded { window, used_micros, ceiling_micros, projected_cost_micros }` under HARD RULE 5 exhaustive-match allowance (new error variant — narrow + necessary)
- `crates/argo-store/tests/spend_ceiling_smoke.rs` (NEW, ≤ 180 LOC) — integration smoke: seed spend rows, check_ceiling at 3 windows, assert remaining math

## Files you must NOT touch

- `crates/argo-store/src/spend.rs` — the record-spend side is stable; this brief is read-only against existing schema
- `crates/argo-cli/src/handlers/judge_eval.rs`, `eval_skill_honesty.rs`, `vertical_eval_judge_backends*.rs`, `argo-agent-runner` — routing callers to the wrapped constructor is the follow-up `mission-budget-gate-route-callers` brief
- `crates/argo-store/migrations/**` — no schema change (reuses existing spend table)
- Any other provider crate beyond `spend_gate.rs`

## Acceptance criteria

- [ ] `CeilingConfig` round-trips through `[budget.ceiling]` TOML with all three fields optional
- [ ] `check_ceiling` reads from existing spend table; computes session / day / month windows from `now()`; returns remaining = None when ceiling = None
- [ ] `SpendGate::complete` returns `ProviderError::BudgetExceeded` before calling inner when projected cost would exceed any set ceiling
- [ ] `SpendGate::complete` is a transparent pass-through when all ceilings are uncapped (config = all None)
- [ ] Projected-cost equal to remaining is ALLOWED (boundary inclusive)
- [ ] Projected-cost exceeding remaining by 1 micro is REJECTED (boundary exclusive)
- [ ] `BudgetWindow` enum has `Session / Day / Month` variants; error message includes which window tripped
- [ ] 7 store unit tests + 5 provider unit tests + 5 config unit tests + 1 integration smoke
- [ ] `cargo clippy -p argo-store -p argo-provider -p argo-config -p argo-types --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-provider -p argo-config -p argo-types` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-llm-budget-hard-ceiling`
- PR title: `provider: hard $$ ceilings per session/day/month at provider-call time (mission-llm-budget-hard-ceiling)`

## Notes for the implementer

- Pattern mirrors the just-merged `mission-circuit-breaker-wire-providers` shape: deliver the wrapper at the construction seam, deliver an alongside-constructor (`from_manifest_with_budget_gate`), queue the routing brief (`mission-budget-gate-route-callers`) as a follow-up. That keeps the may-touch narrow.
- Micros (USD × 1e6) is the established unit — see `argo-store/src/spend.rs` for the convention.
- Time windows: session = lifetime of `SessionId` (no calendar boundary); day = UTC calendar day; month = UTC calendar month. Document the UTC choice in `spend_ceiling.rs` module-level doc.
- `estimated_cost_micros` lookup: use `CompletionRequest::estimated_cost_micros()` if it exists in `argo-types`; if not, the fallback is `40_000` micros (4¢) per request — document this as a deliberately conservative ceiling-side estimate.
- `BudgetExceeded` error: extend the existing `ProviderError` enum; the variant is narrow and necessary. If `BudgetWindow` enum doesn't exist in `argo-types`, add it in the same file.
- Per-tenant scoping: spend queries already filter by `tenant_id`; the gate must call `check_ceiling` with the tenant from `TransportContext`.
- Resist scope creep into per-worker / per-vertical ceilings — that's a separate brief once the per-tenant primitive lands.
