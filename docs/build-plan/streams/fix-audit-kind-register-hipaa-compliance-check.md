# fix-audit-kind-register-hipaa-compliance-check

**Owner**: claude or codex (fast-mode candidate, ~15 LOC).
**Authored against**: origin/main `8e4a9b62d` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-kind-register-hipaa-compliance-check`.
**Source**: evaluator round 78 — `docs/skycot/evals/scheduled/2026-05-26/round-78-gateway-down-hipaa-audit-kind-root-cause.md`.

🔴 **P0 — SHIP-BLOCKER for live demo. The local gateway dies within minutes of boot because the `hipaa_compliance_check` audit kind label is not registered in `AuditKind`, the watchdog trips its `consecutive_failures=3` tamper alarm, and the gateway stops servicing requests.**

## Root cause (traced end-to-end by evaluator round 78)

1. **Producer** — `crates/argo-cli/src/handlers/gateway/hipaa_startup_check.rs:13` writes one audit row at every gateway boot with `kind = "hipaa_compliance_check"`.
2. **Consumer** — `crates/argo-audit/src/event.rs` (`AuditKind::label` / `from_label` / `all`) and `crates/argo-audit/src/event_kind.rs` (variant declarations) have **zero `hipaa` matches** — the label is unregistered.
3. **Watchdog reject** — `crates/argo-audit/src/verify.rs:184-191` calls `AuditKind::from_label(&row.kind)` and emits a `ChainBreak { reason: "unknown audit kind label" }` for every unknown row.
4. **Tamper alarm** — `audit_watchdog` hits its `consecutive_failure_threshold=3` after the first 3-4 boots and `gateway::audit_watchdog audit chain break detected` floods the log until the gateway stops responding.

Re-emitting the row at every boot guarantees the watchdog will trip again 5-30 min after every `scripts/restart-live-gateway.sh`. The restart is a 5-minute reprieve, not a fix.

## Fix recipe (~15 LOC, single crate)

Touch only `crates/argo-audit/`:

1. **`event_kind.rs`** — add one variant at the end of the enum (treat it as additive, before the trailing `}`):

   ```rust
   /// HIPAA deployment-posture breadcrumb emitted at gateway boot
   /// by `argo-cli/src/handlers/gateway/hipaa_startup_check.rs`.
   /// Payload carries `{mode, hipaa_enabled, checked_at, features[]}` —
   /// see `posture_payload()` for the canonical shape.
   HipaaComplianceCheck,
   ```

2. **`event.rs`** — add the three corresponding arms in `label()`, `from_label()`, and `all()`:

   ```rust
   Self::HipaaComplianceCheck => "hipaa_compliance_check",
   ```
   ```rust
   "hipaa_compliance_check" => Self::HipaaComplianceCheck,
   ```
   ```rust
   Self::HipaaComplianceCheck,
   ```

3. **`event_tests.rs`** (or wherever round-trip tests live — confirm with `rg "from_label" crates/argo-audit/src/`) — extend the existing round-trip test or add a one-line pin:

   ```rust
   assert_eq!(
       AuditKind::from_label("hipaa_compliance_check"),
       Some(AuditKind::HipaaComplianceCheck),
   );
   ```

That's it. No producer change required (`hipaa_startup_check.rs` keeps its `const KIND: &str = "hipaa_compliance_check"`).

## Acceptance criteria

- [ ] `cargo test -p argo-audit` passes (the existing round-trip / `all().len()` tests already enforce label/variant parity).
- [ ] `rg "hipaa_compliance_check" crates/argo-audit/src/` returns at least one hit in `event.rs` (label + from_label arms) plus the variant in `event_kind.rs`.
- [ ] After fix lands, restart gateway via `bash scripts/restart-live-gateway.sh --no-prompt --force`; tail `~/.argo/logs/gateway-*.log` for 5 min — zero `audit chain break detected` lines, zero `tamper alarm` lines.
- [ ] `curl -sf http://127.0.0.1:7400/health` returns `200` for ≥5 min after restart.
- [ ] `scripts/check-gateway-freshness.sh` reports `FRESH`.

## May-touch

- `crates/argo-audit/src/event_kind.rs` — add one variant
- `crates/argo-audit/src/event.rs` — add three arms (label, from_label, all)
- `crates/argo-audit/src/event_tests.rs` (or equivalent test file) — add one assertion

## Out of scope

- The three downstream SPA-rendering concerns (empty `worker_id`, raw verb display, hipaa rows on marketing tenant) — those live in the sibling brief `fix-audit-unknown-worker-and-hipaa-fixture-rows`, which is currently BLOCKED on this brief and should be UNBLOCKED + re-scoped after this lands.
- Suppressing the boot-time emit on non-HIPAA tenants — the row is intentional posture-tracking; it just needs a registered label.
- Watchdog hardening (e.g. quarantine instead of tamper-alarm on unknown labels) — separate brief if desired.
- Demo-seed roster / verb mapping work — sibling briefs.

## Authorization fit

Falls under the operator-locked 2026-05-25 overnight `--skip-clippy` ops-* window (expires 22:00Z AEST 2026-05-26 per `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25`). Single-crate additive change, no public surface break.
