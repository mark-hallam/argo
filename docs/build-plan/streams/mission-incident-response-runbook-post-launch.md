# mission-incident-response-runbook-post-launch — post-launch incident response playbook (P2)

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `37f268a6` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-incident-response-runbook-post-launch`.
**Source**: 2026-05-17 audit blind-spot — pre-launch war-room runbook exists (#1765 launch-coordination); post-launch ongoing-operations playbook does not. Customers will hit issues; operator needs muscle-memory before they do.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — incident response is a reliability property, not just a one-time launch concern.
- **Observability** (#2) — runbook references the dashboards + audit-chain + doctor checks operator built (Prometheus #1775, doctor probes #1788+#1804, audit verify-chain).

## Scope

P2. Operator-facing runbook covering the 6 most-likely post-launch incident classes. Each section: detection signal → triage steps → mitigation → rollback path → post-incident notes.

### Incident classes

1. **Gateway 5xx spike**
   - Detection: Prometheus `argo_request_errors_total > 1%` over 5min OR `/healthz` returns non-200.
   - Triage: `argo doctor --network --json` + `tail -100 ~/.argo/gateway.log` + `argo audit verify-chain`.
   - Mitigation: identify failing route from metrics, deploy hotfix OR rollback.
   - Rollback: `flyctl releases list && flyctl deploy --image <prior-image-sha>`.

2. **Magic-link email delivery failure**
   - Detection: Postmark/SendGrid webhook reports bounce rate > 5% OR support tickets mention "didn't get email."
   - Triage: check provider dashboard, sender domain SPF/DKIM, `[email.outbound]` config.
   - Mitigation: failover to backup provider OR pause signup at SPA edge.
   - Rollback: revert to prior provider config.

3. **Stripe webhook failures**
   - Detection: `argo billing dlq list` reports unresolved events OR Stripe dashboard shows delivery failures.
   - Triage: inspect specific `event_id` in DLQ; check signature verification; check tenant table state.
   - Mitigation: `argo billing replay --event-id <id>` after fixing root cause.
   - Rollback: not usually rollbackable (Stripe state is upstream).

4. **Audit chain break**
   - Detection: `argo audit verify-chain --session <id>` reports a break OR Prometheus `argo_audit_chain_breaks_total > 0`.
   - Triage: which session, what kind of break, when did it happen.
   - Mitigation: DO NOT delete the broken row. Document in incident log + escalate. Chain breaks are evidence-quality issues.
   - Rollback: not applicable; preserve as-is for investigation.

5. **Quota / rate-limit incidents**
   - Detection: Prometheus `argo_metric_cardinality_capped_total` non-zero OR `tenant_rate_limit` hits spike.
   - Triage: which tenant, which connector, what's the offending code path.
   - Mitigation: temporary rate-limit override OR per-tenant cap raise OR identify-and-fix the buggy worker action.
   - Rollback: revert temporary rate-limit overrides after fix.

6. **Tenant data isolation alert**
   - Detection: any cross-tenant capability denial spike OR support ticket mentioning "I see someone else's data."
   - Triage: 🔴 SEVERITY-1. Open the multi-tenant isolation test (`cargo test multi_tenant_isolation_load`), check audit chain for cross-tenant rows, check capability denial counts.
   - Mitigation: emergency pause of affected tenants pending root cause.
   - Rollback: not applicable; emergency-pause workspace to halt any further write.

### Plus: shared infrastructure

- On-call rotation template (operator + 1-2 alternates).
- Pager + Slack channel + status page update flow.
- Customer communications template (status page post, email, in-product banner).
- Post-incident review template (PIR — what happened, what we did, what we'll do differently).

LOC: ~400 (operator-facing markdown).

## Files you may touch

- `docs/skycot/operator/incident-response.md` (NEW) — main runbook. ≤200 LOC.
- `docs/skycot/operator/incident-playbooks/gateway-5xx.md` (NEW). ≤60 LOC.
- `docs/skycot/operator/incident-playbooks/magic-link-failure.md` (NEW). ≤50 LOC.
- `docs/skycot/operator/incident-playbooks/stripe-webhook-failure.md` (NEW). ≤50 LOC.
- `docs/skycot/operator/incident-playbooks/audit-chain-break.md` (NEW). ≤50 LOC.
- `docs/skycot/operator/incident-playbooks/quota-rate-limit.md` (NEW). ≤50 LOC.
- `docs/skycot/operator/incident-playbooks/tenant-isolation-alert.md` (NEW). ≤60 LOC.
- `docs/skycot/operator/post-incident-review-template.md` (NEW). ≤40 LOC.
- `docs/skycot/operator/customer-comms-templates.md` (NEW). ≤60 LOC.

## Files you must NOT touch

- Code surfaces — pure operator runbook.
- The launch war-room runbook (#1765) — separate concern (pre-launch + launch-day, not post-launch ongoing).
- Architecture diagrams — read-only references.

## Acceptance criteria

- [ ] Main runbook + 6 incident-playbook files + PIR template + customer-comms templates all present.
- [ ] Each playbook follows the same shape: detection → triage → mitigation → rollback → notes.
- [ ] Every playbook cites specific tools or commands operator built (Prometheus / doctor / audit verify-chain / billing replay / etc.).
- [ ] Customer-comms templates have placeholders for severity / impact / ETA / next-update.
- [ ] PIR template captures what happened + 5-whys + action items.
- [ ] Operator can read the runbook end-to-end in < 30 min.
- [ ] No code touched.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-incident-response-runbook-post-launch`
- Title: `docs: post-launch incident response runbook (6 playbooks + PIR + comms) (mission-incident-response-runbook-post-launch)`
- Worker verification: docs-only — no clippy/test gate. Confirm markdown parses cleanly. Per SWARM_PROTOCOL.md.

## Hidden constraints

- **Reference what we built.** Every command in a playbook must exist on main (not a TODO). Cross-check `argo doctor`, `argo audit verify-chain`, `argo billing replay`, `argo billing dlq` against actual CLI surface.
- **Tone: calm + competent + helpful** per IDENTITY.md. Operator reads this during an incident; don't add emojis or dramatic language.
- **No real customer data in examples.** Use placeholders like `<tenant-acme>` not `tenant-real-customer-xyz`.
- **Severity classification.** Each playbook tags severity (P0 = data leak / chain break; P1 = revenue impact; P2 = single-tenant issue). PIR template uses same scale.
- **Status page integration.** Reference the status page from #1760 — link to it, don't reinvent.
- **Closes audit blind-spot** for customer-ops + post-launch operations.
